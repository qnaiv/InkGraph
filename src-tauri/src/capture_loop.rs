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

/// キャプチャループのメインエントリポイント
pub async fn run(app: AppHandle, state: AppState, window_title: String) {
    log::info!("[capture_loop] starting for window: {window_title}");

    // ウィンドウタイトルを state に保存
    {
        let mut target = state.target_window.lock().await;
        *target = Some(window_title.clone());
    }

    // キャプチャ状態をフロントエンドへ通知
    let _ = app.emit(
        "capture_status",
        CaptureStatusPayload {
            active: true,
            fps: 0.0,
            window_title: Some(window_title.clone()),
        },
    );

    #[cfg(target_os = "windows")]
    {
        run_windows_loop(&app, &state, &window_title).await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 非 Windows: ダミーループ (CI / 開発確認用)
        run_stub_loop(&app, &state).await;
    }

    // 停止通知
    let _ = app.emit(
        "capture_status",
        CaptureStatusPayload {
            active: false,
            fps: 0.0,
            window_title: None,
        },
    );

    log::info!("[capture_loop] stopped");
}

// ---------------------------------------------------------------------------
// Windows 実装
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
async fn run_windows_loop(app: &AppHandle, state: &AppState, window_title: &str) {
    use crate::{
        capture::{capture_window_frame, list_capturable_windows},
        detector::{DetectionResult, ResultDetector},
        extractor::extract_match_data,
        db::new_match_from_ocr,
    };

    // 対象ウィンドウの HWND を解決
    let hwnd = match find_hwnd_by_title(window_title) {
        Some(h) => h,
        None => {
            log::error!("[capture_loop] window not found: {window_title}");
            return;
        }
    };

    let mut detector = ResultDetector::new(30);
    let interval = Duration::from_millis(200); // 5fps

    loop {
        // 停止チェック
        if !*state.is_capturing.lock().await {
            break;
        }

        // フレーム取得
        let frame = match capture_window_frame(hwnd) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("[capture_loop] capture failed: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        // WIN/LOSE 検知
        let detection = match detector.detect(&frame) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("[capture_loop] detection failed: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        match detection {
            DetectionResult::NotDetected => {}
            result => {
                let result_str = if result == DetectionResult::Win { "win" } else { "lose" };

                // データ抽出
                match extract_match_data(&frame, result_str) {
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
                            MatchDetectedPayload {
                                match_data: match_record,
                                ocr_confidence: 1.0,
                            },
                        );
                    }
                    Err(e) => {
                        log::error!("[capture_loop] extraction failed: {e}");
                    }
                }
            }
        }

        tokio::time::sleep(interval).await;
    }
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

    // ダミー試合データを送信
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
        MatchDetectedPayload {
            match_data: dummy,
            ocr_confidence: 0.0,
        },
    );

    // その後は停止を待つ
    loop {
        if !*state.is_capturing.lock().await {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
