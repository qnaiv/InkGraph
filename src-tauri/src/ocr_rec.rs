/// InkGraph — PP-OCRv5 認識モデル (ORT 直接実行)
///
/// YOLO が提供するテキスト BBox のクロップ画像を PP-OCRv5 server rec で認識する。
/// DBNet 検出は不要 (YOLO が BBox を提供するため)。
///
/// 前処理: BGRA → RGB (アルファ除去 + B↔R スワップ)、h=48 リサイズ、
///         (pixel/255.0 - 0.5) / 0.5 正規化、CHW テンソル
/// CTC: PP-OCRv5 規約 — index 0 = blank、index i → dict[i-1]

use std::{path::Path, sync::{Mutex, OnceLock}};
use anyhow::{anyhow, bail, Result};
use image::{ImageBuffer, Rgb, imageops};
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::Tensor};

struct RecEngine {
    session:    Mutex<Session>, // run() は &mut self なので Mutex が必要
    input_name: String,
    max_w:      usize,   // 0 = 動的幅; >0 = 静的最大幅 (右側ゼロパディング)
    dict:       Vec<String>,
}

static ENGINE: OnceLock<RecEngine> = OnceLock::new();

// SessionBuilder のエラー型が Send+Sync でないため、ort::Result 内でチェーンして
// 境界で anyhow::Error へ変換する
fn build_session(rec_path: &Path) -> ort::Result<Session> {
    Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(2)?
        .commit_from_file(rec_path)
}

pub fn init(rec_path: &Path, dict_path: &Path) -> Result<()> {
    let session = build_session(rec_path)
        .map_err(|e| anyhow!("rec model load failed: {e}"))?;

    let input_name = session.inputs()[0].name().to_string();
    let max_w = 0usize; // PP-OCRv5 server rec は動的幅でエクスポートが標準
    log::info!("[ocr_rec] input_name={input_name}, max_w={max_w}");

    let dict: Vec<String> = std::fs::read_to_string(dict_path)?
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    log::info!("[ocr_rec] dict size={}", dict.len());

    ENGINE
        .set(RecEngine { session: Mutex::new(session), input_name, max_w, dict })
        .map_err(|_| anyhow!("RecEngine already initialized"))?;
    log::info!("[ocr_rec] rec model loaded: {}", rec_path.display());
    Ok(())
}

/// BGRA クロップ画像を PP-OCRv5 rec モデルで認識して生テキストを返す。
///
/// チャンネル順は BGR のまま (PaddleOCR は BGR で訓練)。
/// OCR 結果が文字化けする場合、ステップ 1 の channel 順序を反転 (RGB) して試すこと。
pub fn recognize_bgra(bgra: &[u8], width: u32, height: u32) -> Result<String> {
    let engine = ENGINE.get().ok_or_else(|| anyhow!("ocr_rec not initialized"))?;

    // 1. BGRA → RGB (アルファ除去 + B↔R スワップ)
    //    このモデルは RGB 入力を期待している
    let bgr: Vec<u8> = bgra.chunks(4)
        .flat_map(|p| [p[2], p[1], p[0]])
        .collect();

    // 2. h=48 固定でアスペクト比維持リサイズ
    let src = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width, height, bgr)
        .ok_or_else(|| anyhow!("ImageBuffer creation failed ({}x{})", width, height))?;
    let target_w = ((width as f32 * 48.0 / height.max(1) as f32).round() as usize).max(1);
    let resized = imageops::resize(&src, target_w as u32, 48, imageops::FilterType::Triangle);

    // 3. CHW f32 正規化: (byte/255.0 - 0.5) / 0.5 → [-1.0, 1.0]
    //    静的幅モデルの場合は右側を -1.0 (黒) でパディング
    let final_w = if engine.max_w > 0 { engine.max_w.max(target_w) } else { target_w };
    let mut chw = vec![-1.0f32; 3 * 48 * final_w]; // デフォルト -1.0 = 正規化黒
    for y in 0..48usize {
        for x in 0..target_w {
            let px = resized.get_pixel(x as u32, y as u32);
            for c in 0..3usize {
                chw[c * 48 * final_w + y * final_w + x] =
                    (px[c] as f32 / 255.0 - 0.5) / 0.5;
            }
        }
    }

    // 4. ORT 推論
    //    入力名 "x" は PP-OCRv5 SVTR_HGNet の標準名。
    //    初期化ログで input_name を確認し、異なる場合はここを修正する。
    let tensor = Tensor::<f32>::from_array((vec![1usize, 3, 48, final_w], chw))?;
    // MutexGuard を変数に束縛して outputs がそれを借用できるようにする
    let mut guard = engine.session
        .lock()
        .map_err(|_| anyhow!("session mutex poisoned"))?;
    let outputs = guard.run(ort::inputs!["x" => tensor])?;

    // 5. 出力テンソル: [1, seq_len, num_classes]
    let (shape, data) = outputs[0].try_extract_tensor::<f32>()?;
    if shape.len() != 3 || shape[0] != 1 {
        bail!("unexpected rec output shape: {:?}", shape);
    }
    let seq_len     = shape[1] as usize;
    let num_classes = shape[2] as usize;

    Ok(ctc_decode(&data, seq_len, num_classes, &engine.dict))
}

/// CTC グリーディデコード。
/// PP-OCRv5 規約: index 0 = blank token、index i (i≥1) → dict[i-1]。
fn ctc_decode(data: &[f32], seq_len: usize, num_classes: usize, dict: &[String]) -> String {
    let mut result = String::new();
    let mut prev   = usize::MAX;
    for t in 0..seq_len {
        let frame = &data[t * num_classes..(t + 1) * num_classes];
        let idx = frame
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        if idx != 0 && idx != prev {
            if let Some(ch) = dict.get(idx.saturating_sub(1)) {
                result.push_str(ch);
            }
        }
        prev = idx;
    }
    result
}
