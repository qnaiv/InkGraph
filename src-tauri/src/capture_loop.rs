/// IkaVision XP — キャプチャループ
///
/// バックグラウンドタスクとして動作し、対象ウィンドウを監視します。
/// WIN/LOSE を検知したらデータを抽出して DB に保存し、
/// フロントエンドへイベントを送信します。

use std::time::Duration;
use tauri::{AppHandle, Emitter};
use crate::{
    state::AppState,
    types::CaptureStatusPayload,
};

/// キャプチャループのメインエントリポイント。
/// hwnd を直接受け取り、タイトルマッチは行わない。
pub async fn run(app: AppHandle, state: AppState, hwnd: u64) {
    log::info!("[capture_loop] starting for hwnd={hwnd}");

    #[cfg(target_os = "windows")]
    {
        run_windows_loop(&app, &state, hwnd).await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        run_stub_loop(&app, &state).await;
    }

    // タスクが自然終了した場合の後処理
    // (stop_capture による abort の場合はここに到達しない)
    *state.is_capturing.lock().await = false;
    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: false,
        fps: 0.0,
        window_title: None,
    });

    log::info!("[capture_loop] stopped");
}

// ---------------------------------------------------------------------------
// Windows 実装
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
async fn run_windows_loop(app: &AppHandle, state: &AppState, hwnd: u64) {
    use crate::{
        capture::WindowCaptureSession,
        db::new_match_from_ocr,
        detector::{DetectionResult, ResultDetector},
        extractor::extract_match_data,
        types::MatchDetectedPayload,
    };

    // WGC セッションをループ外で一度だけ作成
    let session = match tokio::task::block_in_place(|| WindowCaptureSession::new(hwnd)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[capture_loop] WGC session failed: {e}");
            // セッション作成失敗をフロントエンドに通知
            let _ = app.emit("capture_status", CaptureStatusPayload {
                active: false,
                fps: 0.0,
                window_title: None,
            });
            return;
        }
    };

    // セッション準備完了後にキャプチャ中を通知
    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: true,
        fps: 5.0,
        window_title: None,
    });

    let mut detector  = ResultDetector::new(30);
    let interval      = Duration::from_millis(200); // 5 fps
    let mut frame_count: u64 = 0;

    loop {
        if !*state.is_capturing.lock().await {
            break;
        }

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

// ---------------------------------------------------------------------------
// 非 Windows スタブ (CI / macOS 開発確認用)
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
async fn run_stub_loop(app: &AppHandle, state: &AppState) {
    use crate::{db::new_match_from_ocr, types::MatchDetectedPayload};

    log::warn!("[capture_loop] running stub loop (non-Windows)");

    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: true,
        fps: 0.0,
        window_title: None,
    });

    tokio::time::sleep(Duration::from_secs(3)).await;

    let dummy = new_match_from_ocr(
        "win",
        Some(5), Some(1), Some(2), Some(2341.5),
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
