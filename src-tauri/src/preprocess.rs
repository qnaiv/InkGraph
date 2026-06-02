/// InkGraph — 画像前処理モジュール
///
/// YOLO 入力用のレターボックスリサイズと、
/// WinRT OCR 精度向上のための白文字抽出・二値化を提供する。

use anyhow::Result;
use image::{DynamicImage, GrayImage, ImageBuffer, Rgb, RgbImage, RgbaImage};

// ---------------------------------------------------------------------------
// BGRA8 ↔ image クレート 変換
// ---------------------------------------------------------------------------

/// 内部 BGRA8 バッファを `image::RgbaImage` に変換する。
pub fn bgra_to_rgba_image(bgra: &[u8], width: u32, height: u32) -> RgbaImage {
    let mut rgba = Vec::with_capacity(bgra.len());
    for chunk in bgra.chunks_exact(4) {
        rgba.push(chunk[2]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[0]); // B
        rgba.push(chunk[3]); // A
    }
    ImageBuffer::from_raw(width, height, rgba)
        .expect("buffer size must match width*height*4")
}

/// 内部 BGRA8 バッファを `image::RgbImage` に変換する。
pub fn bgra_to_rgb_image(bgra: &[u8], width: u32, height: u32) -> RgbImage {
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for chunk in bgra.chunks_exact(4) {
        rgb.push(chunk[2]); // R
        rgb.push(chunk[1]); // G
        rgb.push(chunk[0]); // B
    }
    ImageBuffer::from_raw(width, height, rgb)
        .expect("buffer size must match width*height*3")
}

// ---------------------------------------------------------------------------
// YOLO 入力用: レターボックスリサイズ
// ---------------------------------------------------------------------------

/// レターボックスリサイズのパラメータ
#[derive(Debug, Clone, Copy)]
pub struct LetterboxParams {
    /// min(target_size/fw, target_size/fh) のスケール係数
    pub scale:   f32,
    /// x 方向パディング (YOLO 入力空間 px)
    pub pad_x:   f32,
    /// y 方向パディング (YOLO 入力空間 px)
    pub pad_y:   f32,
    /// 元フレームの幅 (px)
    pub frame_w: u32,
    /// 元フレームの高さ (px)
    pub frame_h: u32,
}

impl LetterboxParams {
    /// YOLO 出力の bbox (cx,cy,bw,bh は YOLO 入力 px) を
    /// 元フレームの正規化座標 [0,1] に変換する。
    ///
    /// 変換式:
    ///   x1_norm = ((cx - bw/2) - pad_x) / (scale * frame_w)
    pub fn to_normalized(&self, cx: f32, cy: f32, bw: f32, bh: f32) -> (f32, f32, f32, f32) {
        let s = self.scale;
        let x1 = ((cx - bw / 2.0) - self.pad_x) / (s * self.frame_w as f32);
        let y1 = ((cy - bh / 2.0) - self.pad_y) / (s * self.frame_h as f32);
        let x2 = ((cx + bw / 2.0) - self.pad_x) / (s * self.frame_w as f32);
        let y2 = ((cy + bh / 2.0) - self.pad_y) / (s * self.frame_h as f32);
        (x1.clamp(0.0, 1.0), y1.clamp(0.0, 1.0), x2.clamp(0.0, 1.0), y2.clamp(0.0, 1.0))
    }
}

/// BGRA8 フレームを `target_size × target_size` にレターボックスリサイズし、
/// CHW 形式 (1 × 3 × H × W) の f32 テンソル (0.0–1.0) と変換パラメータを返す。
///
/// # 戻り値
/// `(flat_chw_vec, LetterboxParams)`
/// flat_chw_vec は CHW 順のフラット f32 配列。(shape, data) タプルとして YOLO へ渡す。
pub fn letterbox_bgra(
    bgra:        &[u8],
    frame_w:     u32,
    frame_h:     u32,
    target_size: u32,
) -> Result<(Vec<f32>, LetterboxParams)> {
    let rgb = bgra_to_rgb_image(bgra, frame_w, frame_h);
    let dyn_img = DynamicImage::ImageRgb8(rgb);

    // アスペクト比を保ちながら target_size に収まるスケール
    let scale = (target_size as f32 / frame_w as f32)
        .min(target_size as f32 / frame_h as f32);
    let new_w = (frame_w as f32 * scale).round() as u32;
    let new_h = (frame_h as f32 * scale).round() as u32;

    let pad_x = (target_size as f32 - new_w as f32) / 2.0;
    let pad_y = (target_size as f32 - new_h as f32) / 2.0;

    // リサイズ
    let resized = dyn_img
        .resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
        .to_rgb8();

    // 128 グレーで初期化 (レターボックス領域)
    let mut canvas = RgbImage::from_pixel(target_size, target_size, Rgb([114, 114, 114]));
    let pad_x_u = pad_x as u32;
    let pad_y_u = pad_y as u32;
    image::imageops::overlay(&mut canvas, &resized, pad_x_u as i64, pad_y_u as i64);

    // HWC → CHW, uint8 → f32 [0,1]
    let ts = target_size as usize;
    let mut chw = vec![0f32; 3 * ts * ts];
    for y in 0..ts {
        for x in 0..ts {
            let px = canvas.get_pixel(x as u32, y as u32);
            chw[0 * ts * ts + y * ts + x] = px[0] as f32 / 255.0; // R
            chw[1 * ts * ts + y * ts + x] = px[1] as f32 / 255.0; // G
            chw[2 * ts * ts + y * ts + x] = px[2] as f32 / 255.0; // B
        }
    }

    let params = LetterboxParams {
        scale,
        pad_x,
        pad_y,
        frame_w,
        frame_h,
    };
    Ok((chw, params))
}

// ---------------------------------------------------------------------------
// OCR 前処理: 白文字抽出 + 二値化
// ---------------------------------------------------------------------------

/// BGRA8 クロップ領域から白い文字を強調した BGRA8 バッファを返す。
///
/// スプラトゥーン3 のリザルト画面はインクエフェクト背景に白文字が重なるため、
/// 「V (明度) が高く S (彩度) が低い = 白系」ピクセルを保持し他を黒に落とす。
pub fn extract_white_text(bgra: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = vec![0u8; bgra.len()];
    for (i, chunk) in bgra.chunks_exact(4).enumerate() {
        let b = chunk[0] as f32;
        let g = chunk[1] as f32;
        let r = chunk[2] as f32;

        let v = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let s = if v > 0.0 { (v - min_c) / v * 255.0 } else { 0.0 };

        // 明度 ≥ 180 かつ 彩度 ≤ 50 を「白系」とみなす
        let is_white = v >= 180.0 && s <= 50.0;
        let val = if is_white { 255u8 } else { 0u8 };

        let base = i * 4;
        out[base]     = val; // B
        out[base + 1] = val; // G
        out[base + 2] = val; // R
        out[base + 3] = 255; // A
    }
    out
}

/// BGRA8 バッファを双線形補間で 2 倍にアップスケールして返す。
/// WinRT OCR は低解像度テキストへの精度が低いため、小クロップ領域に適用する。
pub fn upscale_2x(bgra: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let new_w = width * 2;
    let new_h = height * 2;
    let rgba_img = bgra_to_rgba_image(bgra, width, height);
    let resized  = image::imageops::resize(
        &rgba_img, new_w, new_h, image::imageops::FilterType::Triangle,
    );
    let bgra_out: Vec<u8> = resized.into_raw()
        .chunks_exact(4)
        .flat_map(|c| [c[2], c[1], c[0], c[3]])
        .collect();
    (bgra_out, new_w, new_h)
}

/// グレースケール変換後に Otsu の二値化を適用した BGRA8 バッファを返す。
///
/// `imageproc` の Otsu 実装を利用する。
pub fn otsu_binarize(bgra: &[u8], width: u32, height: u32) -> Vec<u8> {
    // BGRA → グレースケール
    let gray_data: Vec<u8> = bgra
        .chunks_exact(4)
        .map(|c| {
            let b = c[0] as u32;
            let g = c[1] as u32;
            let r = c[2] as u32;
            ((299 * r + 587 * g + 114 * b) / 1000) as u8
        })
        .collect();

    let gray_img: GrayImage =
        ImageBuffer::from_raw(width, height, gray_data).expect("size mismatch");

    let threshold = imageproc::contrast::otsu_level(&gray_img);
    let bin = imageproc::contrast::threshold(&gray_img, threshold, imageproc::contrast::ThresholdType::Binary);

    // GrayImage → BGRA8
    bin.into_raw()
        .iter()
        .flat_map(|&v| [v, v, v, 255u8])
        .collect()
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bgra_pixel(r: u8, g: u8, b: u8) -> Vec<u8> {
        vec![b, g, r, 255]
    }

    #[test]
    fn test_bgra_to_rgb_image_swaps_channels() {
        let bgra = make_bgra_pixel(200, 100, 50); // R=200, G=100, B=50
        let img = bgra_to_rgb_image(&bgra, 1, 1);
        let px = img.get_pixel(0, 0);
        assert_eq!(px[0], 200, "R");
        assert_eq!(px[1], 100, "G");
        assert_eq!(px[2], 50,  "B");
    }

    #[test]
    fn test_extract_white_text_keeps_white() {
        let bgra = make_bgra_pixel(240, 240, 240); // 白
        let out = extract_white_text(&bgra, 1, 1);
        assert_eq!(out[0], 255); // 白として保持
    }

    #[test]
    fn test_extract_white_text_drops_colored() {
        let bgra = make_bgra_pixel(200, 50, 50); // 高彩度の赤
        let out = extract_white_text(&bgra, 1, 1);
        assert_eq!(out[0], 0); // 黒に落ちる
    }

    #[test]
    fn test_letterbox_output_size() {
        let bgra = vec![0u8; 1920 * 1080 * 4];
        let (chw, params) = letterbox_bgra(&bgra, 1920, 1080, 640).unwrap();
        assert_eq!(chw.len(), 3 * 640 * 640);
        assert!(params.scale > 0.0);
        assert!(params.pad_x >= 0.0);
        assert_eq!(params.frame_w, 1920);
        assert_eq!(params.frame_h, 1080);
    }

    #[test]
    fn test_letterbox_normalized_roundtrip() {
        // 1920×1080 → 640: scale=640/1920≈0.333, pad_y=(640-360)/2=140
        let bgra = vec![0u8; 1920 * 1080 * 4];
        let (_, params) = letterbox_bgra(&bgra, 1920, 1080, 640).unwrap();

        // 画像中央の bbox
        let (x1, y1, x2, y2) = params.to_normalized(320.0, 320.0, 64.0, 64.0);
        // 元画像の正規化座標に収まっているはず
        assert!(x1 >= 0.0 && x2 <= 1.0, "x out of range: {x1}..{x2}");
        assert!(y1 >= 0.0 && y2 <= 1.0, "y out of range: {y1}..{y2}");
    }
}
