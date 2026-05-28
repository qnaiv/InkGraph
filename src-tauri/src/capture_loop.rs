/// IkaVision XP — キャプチャループ
///
/// `WindowCaptureSession` を一度だけ作成してループ全体で保持する。
/// これにより WGC キャプチャインジケーターが点滅せず、
/// フレームも安定して取得できる。

use std::time::Duration;
use tauri::{AppHandle, Emitter};
use crate::{
    state::AppState,
    types::CaptureStatusPayload,
};

pub async fn run(app: AppHandle, state: AppState, window_title: String) {
    log::info!("[capture_loop] starting for window: {window_title}");

    {
        let mut target = state.target_window.lock().await;
        *target = Some(window_title.clone());
    }

    let _ = app.emit(
        "capture_status",
        CaptureStatusPayload { active: true, fps: 0.0, window_title: Some(window_title.clone()) },
    );

    #[cfg(target_os = "windows")]
    {
        run_windows_loop(&app, &state, &window_title).await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        run_stub_loop(&app, &state).await;
    }

    let _ = app.emit(
        "capture_status",
        CaptureStatusPayload { active: false, fps: 0.0, window_title: None },
    );

    log::info!("[capture_loop] stopped");
}

// ---------------------------------------------------------------------------
// Windows 実装
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
async fn run_windows_loop(app: &AppHandle, state: &AppState, window_title: &str) {
    use crate::{
        capture::WindowCaptureSession,
        db::new_match_from_ocr,
        detector::{DetectionResult, ResultDetector},
        extractor::extract_match_data,
        types::MatchDetectedPayload,
    };

    let hwnd = match find_hwnd_by_title(window_title) {
        Some(h) => h,
        None => {
            log::error!("[capture_loop] window not found: {window_title}");
            return;
        }
    };

    // セッションはループ外で一度だけ作成する (これがポイント)
    let session = match tokio::task::block_in_place(|| WindowCaptureSession::new(hwnd)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[capture_loop] failed to create WGC session: {e}");
            return;
        }
    };

    let mut detector = ResultDetector::new(30);
    let interval     = Duration::from_millis(200); // 5 fps
    let mut frame_count: u64 = 0;

    loop {
        if !*state.is_capturing.lock().await {
            break;
        }

        // フレーム取得 (ブロッキング処理を Tokio に伝える)
        let frame = match tokio::task::block_in_place(|| session.get_frame()) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("[capture_loop] get_frame failed: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        frame_count += 1;
        if frame_count % 25 == 0 {
            log::debug!("[capture_loop] frame #{frame_count} {}x{}", frame.width, frame.height);
        }

        // WIN/LOSE 検知
        let detection = match detector.detect(&frame) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("[capture_loop] detection error: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        if let Some(result_str) = detection.result_str() {
            let arrow_y = detection.arrow_y_ratio().unwrap_or(0.44);
            match extract_match_data(&frame, result_str, arrow_y) {
                Ok(data) => {
                    let match_record = new_match_from_ocr(
                        &data.result,
                        data.kill_count,
                        data.assist_count,
                        data.death_count,
                        data.xp_after,
                        data.rule,
                        data.stage,
                    );
                    log::info!("[capture_loop] match detected: {:?}", match_record);
                    let _ = app.emit(
                        "match_detected",
                        MatchDetectedPayload { match_data: match_record, ocr_confidence: 1.0 },
                    );
                }
                Err(e) => {
                    log::error!("[capture_loop] extraction failed: {e}");
                }
            }
        }

        tokio::time::sleep(interval).await;
    }
    // session は Drop で自動的に Close される
}

#[cfg(target_os = "windows")]
fn find_hwnd_by_title(title: &str) -> Option<u64> {
    use crate::capture::list_capturable_windows;
    list_capturable_windows()
        .ok()?
        .into_iter()
        .find(|w| w.title.contains(title))
        .map(|w| w.hwnd)
}

// ---------------------------------------------------------------------------
// 非 Windows スタブ (CI / macOS 開発確認用)
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
async fn run_stub_loop(app: &AppHandle, state: &AppState) {
    use crate::{db::new_match_from_ocr, types::MatchDetectedPayload};

    log::warn!("[capture_loop] running stub loop (non-Windows)");
    tokio::time::sleep(Duration::from_secs(3)).await;

    let dummy = new_match_from_ocr(
        "win",
        Some(5),
        Some(1),
        Some(2),
        Some(2341.5),
        Some("ガチエリア".to_string()),
        Some("マテガイ放水路".to_string()),
    );
    let _ = app.emit(
        "match_detected",
        MatchDetectedPayload { match_data: dummy, ocr_confidence: 0.0 },
    );

    loop {
        if !*state.is_capturing.lock().await { break; }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
