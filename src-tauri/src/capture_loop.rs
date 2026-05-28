/// InkGraph — キャプチャループ
///
/// バックグラウンドタスクとして動作し、対象ウィンドウを監視します。
///
/// 検知フロー:
///   BattleStarted  → "in_progress" レコードを DB に作成し `battle_started` イベントを送信
///   Win / Lose     → 既存の "in_progress" レコードを更新し `match_detected` イベントを送信

use std::time::Duration;
use tauri::{AppHandle, Emitter};
use crate::{
    state::AppState,
    types::CaptureStatusPayload,
};

/// キャプチャループのメインエントリポイント。
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
        db::{new_in_progress_match, new_match_from_ocr},
        detector::{DetectionResult, ResultDetector},
        extractor::extract_match_data,
        types::MatchDetectedPayload,
    };

    let session = match tokio::task::block_in_place(|| WindowCaptureSession::new(hwnd)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[capture_loop] WGC session failed: {e}");
            let _ = app.emit("capture_status", CaptureStatusPayload {
                active: false, fps: 0.0, window_title: None,
            });
            return;
        }
    };

    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: true, fps: 5.0, window_title: None,
    });

    let mut detector = ResultDetector::new();
    let interval     = Duration::from_millis(200); // 5 fps

    // バトル開始時に作成した "in_progress" レコードの ID を保持する。
    // リザルト検知時にこの ID を使って既存レコードを上書きする。
    let mut pending_match_id: Option<String> = None;
    let mut frame_count: u64 = 0;

    loop {
        if !*state.is_capturing.lock().await { break; }

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

        // detect() は Phase 1 で blocking OCR を呼ぶため block_in_place でラップ
        let detection = match tokio::task::block_in_place(|| detector.detect(&frame)) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("[capture_loop] detection error: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        match &detection {
            DetectionResult::BattleStarted => {
                // 既に in_progress を追跡中なら新規作成しない (Phase 1 連続発火の防止)
                if pending_match_id.is_none() {
                    let m = new_in_progress_match();
                    pending_match_id = Some(m.id.clone());
                    log::info!("[capture_loop] battle started → pending_id={}", m.id);
                    let _ = app.emit("battle_started", MatchDetectedPayload {
                        match_data: m,
                        ocr_confidence: 1.0,
                    });
                }
            }

            DetectionResult::Win { arrow_y_ratio } | DetectionResult::Lose { arrow_y_ratio } => {
                let result_str = detection.result_str().unwrap();
                match extract_match_data(&frame, result_str, *arrow_y_ratio) {
                    Ok(data) => {
                        // pending_match_id を使うことで既存の "in_progress" レコードを上書きする
                        let id = pending_match_id.take();
                        let match_record = new_match_from_ocr(
                            id,
                            &data.result,
                            data.kill_count,
                            data.assist_count,
                            data.death_count,
                            data.xp_after,
                            data.rule,
                            data.stage,
                        );
                        log::info!("[capture_loop] match detected id={}: {:?}", match_record.id, match_record.result);
                        let _ = app.emit("match_detected", MatchDetectedPayload {
                            match_data: match_record,
                            ocr_confidence: 1.0,
                        });
                    }
                    Err(e) => {
                        log::error!("[capture_loop] extraction failed: {e}");
                        // pending_match_id はそのまま保持し次のリザルト検知で再試行
                    }
                }
            }

            DetectionResult::NotDetected => {}
        }

        tokio::time::sleep(interval).await;
    }
}

// ---------------------------------------------------------------------------
// 非 Windows スタブ (CI / macOS 開発確認用)
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
async fn run_stub_loop(app: &AppHandle, state: &AppState) {
    use crate::{
        db::{new_in_progress_match, new_match_from_ocr},
        types::MatchDetectedPayload,
    };

    log::warn!("[capture_loop] running stub loop (non-Windows)");

    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: true, fps: 0.0, window_title: None,
    });

    // バトル開始をシミュレート
    tokio::time::sleep(Duration::from_secs(2)).await;
    let in_progress = new_in_progress_match();
    let pending_id  = in_progress.id.clone();
    let _ = app.emit("battle_started", MatchDetectedPayload {
        match_data: in_progress,
        ocr_confidence: 0.0,
    });

    // リザルトをシミュレート
    tokio::time::sleep(Duration::from_secs(3)).await;
    let result = new_match_from_ocr(
        Some(pending_id),
        "win",
        Some(5), Some(1), Some(2), Some(2341.5),
        Some("ガチエリア".to_string()),
        Some("マテガイ放水路".to_string()),
    );
    let _ = app.emit("match_detected", MatchDetectedPayload {
        match_data: result,
        ocr_confidence: 0.0,
    });

    loop {
        if !*state.is_capturing.lock().await { break; }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
