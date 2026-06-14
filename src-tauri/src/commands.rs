/// InkGraph — Tauri コマンド定義

use tauri::{AppHandle, Emitter, State};
use crate::{
    capture::CapturedFrame,
    ocr::ocr_from_file,
    state::AppState,
    types::{CaptureDebugResult, CaptureStatusPayload, OcrTestResult, WindowInfo,
            YoloDebugDetection, FullDebugResult, HeaderDebugResult},
};

// ---------------------------------------------------------------------------
// 内部ヘルパー: YOLO + カスケード推論パイプライン (フレーム取得方法に依存しない)
// ---------------------------------------------------------------------------

/// `frame` を受け取って Model 1 + Model 2 + OCR を実行し FullDebugResult を返す。
/// debug_full / debug_full_from_file の両コマンドから呼ばれる。
#[cfg(target_os = "windows")]
async fn run_full_pipeline(frame: CapturedFrame) -> Result<FullDebugResult, String> {
    use crate::{
        cascade::StatsDetector,
        detector::{YoloDetector, YoloClass},
    };

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();

    let result_model_path = exe_dir.join("assets/models/yolo_result.onnx");
    let stats_model_path  = exe_dir.join("assets/models/yolo_stats.onnx");

    let mut yolo  = YoloDetector::new(&result_model_path);
    let mut stats = StatsDetector::new(&stats_model_path);

    if let Err(e) = tokio::task::block_in_place(|| yolo.load()) {
        return Ok(FullDebugResult {
            frame_w: 0, frame_h: 0,
            model1_loaded: false, model2_loaded: false,
            detections: vec![], ocr: None,
            arrow_found: false,
            crop_x: 0, crop_y: 0, crop_w: 0, crop_h: 0,
            crop_image_base64: None,
            cascade_detections: vec![],
            kill_anchor_x: None, death_anchor_x: None, special_anchor_x: None,
            paint: None, kill: None, death: None, special: None,
            error: Some(format!("yolo_result.onnx ロード失敗: {e}")),
            header: HeaderDebugResult {
                frame_w: 0, frame_h: 0,
                crop_x: 0, crop_y: 0, crop_w: 0, crop_h: 0,
                crop_image_base64: None,
                detections: vec![],
                mode: None, rule: None, stage: None,
                error: None,
            },
        });
    }

    let model2_loaded = tokio::task::block_in_place(|| stats.load()).is_ok();

    let rec_model_path = exe_dir.join("assets/models/ppocr_rec_ja.onnx");
    let dict_path      = exe_dir.join("assets/models/ppocr_dict_ja.txt");
    if let Err(e) = tokio::task::block_in_place(|| crate::ocr_rec::init(&rec_model_path, &dict_path)) {
        log::warn!("[run_full_pipeline] ocr_rec init failed: {e}");
    }

    let (fw, fh) = (frame.width, frame.height);

    let m1_dets = tokio::task::block_in_place(|| yolo.detect_debug(&frame))
        .map_err(|e| format!("YOLO detect failed: {e}"))?;

    let detections: Vec<YoloDebugDetection> = m1_dets.iter().map(|d| YoloDebugDetection {
        class_name: d.class_name.clone(),
        class_id:   d.class_id,
        confidence: d.confidence,
        x1: d.bbox.x1, y1: d.bbox.y1,
        x2: d.bbox.x2, y2: d.bbox.y2,
    }).collect();

    // cascade を先に実行し、KDA 値を extract_debug_ocr に渡す
    // (固定 ROI OCR のハードコード座標ズレを Model 2 の検出結果で補正するため)
    let arrow   = YoloDetector::best_detection(&m1_dets, YoloClass::MyArrow);
    let cascade = tokio::task::block_in_place(|| stats.run_cascade_debug(&frame, arrow));
    let header  = tokio::task::block_in_place(|| stats.run_header_cascade_debug(&frame));

    let cascade_kda = if cascade.kill.is_some() || cascade.death.is_some() || cascade.special.is_some() {
        Some((cascade.kill, cascade.death, cascade.special))
    } else {
        None
    };
    let kda_anchors = Some((cascade.kill_anchor_x, cascade.death_anchor_x, cascade.special_anchor_x));
    let ocr = tokio::task::block_in_place(|| crate::extractor::extract_debug_ocr(&frame, &m1_dets, cascade_kda, kda_anchors));

    Ok(FullDebugResult {
        frame_w: fw, frame_h: fh,
        model1_loaded: true,
        model2_loaded,
        detections,
        ocr: Some(ocr),
        arrow_found: cascade.arrow_found,
        crop_x: cascade.crop_x,
        crop_y: cascade.crop_y,
        crop_w: cascade.crop_w,
        crop_h: cascade.crop_h,
        crop_image_base64: cascade.crop_image_base64,
        cascade_detections: cascade.detections,
        kill_anchor_x: cascade.kill_anchor_x,
        death_anchor_x: cascade.death_anchor_x,
        special_anchor_x: cascade.special_anchor_x,
        paint: cascade.paint,
        kill:  cascade.kill,
        death: cascade.death,
        special: cascade.special,
        error: cascade.error,
        header,
    })
}

// ---------------------------------------------------------------------------
// OCR テストコマンド
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn test_ocr(image_path: String) -> Result<OcrTestResult, String> {
    let result = ocr_from_file(&image_path, None)
        .map_err(|e| format!("OCR failed: {e}"))?;

    let lines: Vec<String> = result
        .text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(OcrTestResult { raw_text: result.text, lines })
}

// ---------------------------------------------------------------------------
// キャプチャ制御コマンド
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_windows() -> Result<Vec<WindowInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        crate::capture::list_capturable_windows()
            .map_err(|e| format!("list_windows failed: {e}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(vec![WindowInfo {
            hwnd: 0,
            title: "[stub] Non-Windows platform".to_string(),
        }])
    }
}

/// キャプチャを開始する。
/// 既存のタスクがあれば abort() してから新しいタスクを起動する。
/// `hwnd` でウィンドウを直接指定することでタイトル曖昧マッチを排除。
#[tauri::command]
pub async fn start_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    hwnd: u64,
) -> Result<(), String> {
    // 既存タスクを abort（再起動時のレースコンディション防止）
    {
        let mut task = state.capture_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }

    *state.is_capturing.lock().await = true;
    log::info!("[commands] start_capture: hwnd={hwnd}");

    let state_clone = state.inner().clone();
    let app_clone   = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        crate::capture_loop::run(app_clone, state_clone, hwnd).await;
    });

    *state.capture_task.lock().await = Some(handle);
    Ok(())
}

// ---------------------------------------------------------------------------
// デバッグキャプチャコマンド
// ---------------------------------------------------------------------------

/// 指定ウィンドウから 1 フレームだけ取得して検知パイプラインの診断情報を返す。
/// リザルト画面を表示した状態で呼ぶと各ステップの通過状況が確認できる。
#[tauri::command]
pub async fn debug_capture(hwnd: u64) -> Result<CaptureDebugResult, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::detector::debug_detect_frame;
        // 専用スレッドで WGC フレームを取得 (Tokio worker スレッドでの FrameArrived デッドロック回避)
        let frame = wgc_capture_on_thread(hwnd).await?;
        debug_detect_frame(&frame).map_err(|e| format!("debug_detect failed: {e}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("debug_capture は Windows 専用です".to_string())
    }
}

/// Model 1 (YOLO) + Model 2 (カスケード) を 1 回のコマンドで実行する統合デバッグ。
/// 信頼度 0.10 以上の全検出を返すため、通常の閾値以下の候補も確認できる。
///
/// # フレーム取得の優先順位
/// 1. キャプチャループが動いていれば `last_frame` を再利用 (WGC 二重セッション回避)
/// 2. ループ停止中は専用スレッドで WGC セッションを作成してフレームを取得
///
/// # WGC / Tokio アパートメント問題について
/// `block_in_place` の中で WGC セッションを作ると、FrameArrived コールバックが
/// Tokio の worker スレッドと同じスレッドに届こうとしてデッドロックすることがある。
/// そのため、セッション作成〜フレーム取得を `std::thread::spawn` で完全に独立した
/// ネイティブスレッドで実行し、結果だけ `oneshot` チャネルで返す設計にしている。
/// もし WGC でタイムアウトが出る場合は `debug_full_from_file` でリザルト画像ファイルを
/// 直接渡すとパイプライン全体をテストできる。
#[tauri::command]
pub async fn debug_full(hwnd: u64, state: State<'_, AppState>) -> Result<FullDebugResult, String> {
    #[cfg(target_os = "windows")]
    {
        // キャプチャループが動いていれば last_frame を再利用して WGC 二重セッションを回避。
        // ループが停止中 (last_frame = None) のみ新しい WGC セッションを専用スレッドで作成する。
        let frame: CapturedFrame = {
            let cached = state.last_frame.lock().await.clone();
            if let Some(f) = cached {
                log::info!("[debug_full] using last_frame from capture loop");
                f
            } else {
                log::info!("[debug_full] capture not running, creating new WGC session on dedicated thread");
                wgc_capture_on_thread(hwnd).await?
            }
        };

        run_full_pipeline(frame).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("debug_full は Windows 専用です".to_string())
    }
}

/// PNG / JPEG などの画像ファイルを入力として YOLO + カスケード全パイプラインを実行する。
///
/// WGC (画面キャプチャ) を使わないため:
/// - キャプチャループが停止中でもデバッグできる
/// - キャプチャカードで撮ったスクリーンショットを直接渡せる
/// - Ubuntu CI でも (YOLO モデルさえあれば) テスト可能
///
/// フロントエンドの「ファイルから診断」ボタンから呼び出すほか、
/// `cargo test` のインテグレーションテストでも使用できる。
#[tauri::command]
pub async fn debug_full_from_file(image_path: String) -> Result<FullDebugResult, String> {
    #[cfg(target_os = "windows")]
    {
        let frame = tokio::task::block_in_place(|| {
            crate::capture::frame_from_file(&image_path)
        })
        .map_err(|e| format!("画像読み込み失敗: {e}"))?;

        log::info!("[debug_full_from_file] loaded {}x{} from {image_path}", frame.width, frame.height);
        run_full_pipeline(frame).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = image_path;
        Err("debug_full_from_file は Windows 専用です".to_string())
    }
}

/// 専用ネイティブスレッドで WGC セッションを作成しフレームを 1 枚取得する。
///
/// Tokio の worker スレッドの中で WinRT の FrameArrived イベントを待つと、
/// コールバックが同じスレッドを待ってデッドロックすることがある。
/// `std::thread::spawn` で完全に独立したスレッドを使うことでこの問題を回避する。
#[cfg(target_os = "windows")]
async fn wgc_capture_on_thread(hwnd: u64) -> Result<CapturedFrame, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<CapturedFrame, String>>();
    std::thread::spawn(move || {
        let result = (|| -> Result<CapturedFrame, String> {
            let session = crate::capture::WindowCaptureSession::new(hwnd)
                .map_err(|e| format!("WGC session failed: {e}"))?;
            for attempt in 0u32..2 {
                match session.get_frame() {
                    Ok(f) => return Ok(f),
                    Err(e) => {
                        if attempt == 1 { return Err(format!("get_frame failed: {e}")); }
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                }
            }
            unreachable!()
        })();
        let _ = tx.send(result);
    });
    rx.await.map_err(|_| "WGC thread panicked".to_string())?
}

/// キャプチャを停止する。
/// タスクを abort() して即座に終了させ、inactive をフロントエンドに通知する。
#[tauri::command]
pub async fn stop_capture(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut task = state.capture_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }

    *state.is_capturing.lock().await = false;

    // abort でタスクが強制終了するため run() 末尾が動かない → ここで通知
    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: false,
        fps: 0.0,
        window_title: None,
        yolo_loaded: false,
    });

    log::info!("[commands] stop_capture");
    Ok(())
}
