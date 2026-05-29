/// InkGraph — キャプチャループ
///
/// 検知フロー:
///
///   ┌─ YOLO パス (yolo_result.onnx が配置済みの場合) ──────────────────────┐
///   │  バトル開始: ピクセル検知 (暗巻物 + OCR)                              │
///   │  リザルト検知: YOLO → BBox クロップ → 白文字抽出 → WinRT OCR         │
///   │  取得項目: 勝ち/負け, ルール, ステージ, モード, キル/デス/SP          │
///   └──────────────────────────────────────────────────────────────────────┘
///   ┌─ ピクセルフォールバック (モデル未配置の場合) ─────────────────────────┐
///   │  バトル開始: ピクセル検知                                              │
///   │  リザルト検知: グレー行 + 黄色矢印ピクセル判定 → 固定 ROI OCR        │
///   │  取得項目: 勝ち/負け, ルール, ステージ, キル/デス/SP (モードなし)     │
///   └──────────────────────────────────────────────────────────────────────┘

use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use crate::{
    state::AppState,
    types::CaptureStatusPayload,
};

/// キャプチャループのメインエントリポイント
pub async fn run(app: AppHandle, state: AppState, hwnd: u64) {
    log::info!("[capture_loop] starting for hwnd={hwnd}");

    #[cfg(target_os = "windows")]
    { run_windows_loop(&app, &state, hwnd).await; }

    #[cfg(not(target_os = "windows"))]
    { run_stub_loop(&app, &state).await; }

    *state.is_capturing.lock().await = false;
    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: false, fps: 0.0, window_title: None,
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
        detector::{DetectionResult, ResultDetector, YoloClass, YoloDetector},
        extractor::{extract_from_yolo_detections, extract_match_data},
        types::MatchDetectedPayload,
    };

    // ── YOLO モデルをロード (失敗してもフォールバックで継続) ──────────────
    // 探索順: 実行ファイルと同ディレクトリ → カレントディレクトリ
    let model_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("assets/models/yolo_result.onnx")))
        .unwrap_or_else(|| std::path::PathBuf::from("assets/models/yolo_result.onnx"));

    let mut yolo = YoloDetector::new(&model_path);
    match tokio::task::block_in_place(|| yolo.load()) {
        Ok(()) => log::info!("[capture_loop] YOLO loaded: {}", model_path.display()),
        Err(e) => log::warn!("[capture_loop] YOLO not loaded → pixel fallback: {e}"),
    }

    // ── WGC セッション作成 ─────────────────────────────────────────────────
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

    let mut detector           = ResultDetector::new();
    let interval               = Duration::from_millis(200); // 5 fps
    let mut pending_match_id: Option<String> = None;
    let mut frame_count: u64   = 0;
    // バトル開始後にリザルト誤検知を防ぐ冷却時間
    let mut battle_started_at: Option<Instant> = None;
    const RESULT_COOLDOWN_SECS: u64 = 15;

    loop {
        if !*state.is_capturing.lock().await { break; }

        let frame = match tokio::task::block_in_place(|| session.get_frame()) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("[capture_loop] get_frame: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        frame_count += 1;
        if frame_count % 25 == 0 {
            log::debug!("[capture_loop] frame #{frame_count} {}x{}", frame.width, frame.height);
        }

        // ══════════════════════════════════════════════════════════════════
        // YOLO パス
        // ══════════════════════════════════════════════════════════════════
        if yolo.is_loaded() {
            if detector.is_waiting() {
                // --- バトル開始をピクセル検知 ---
                let det = match tokio::task::block_in_place(|| detector.detect(&frame)) {
                    Ok(d)  => d,
                    Err(e) => {
                        log::warn!("[capture_loop] pixel detect: {e}");
                        tokio::time::sleep(interval).await;
                        continue;
                    }
                };
                if matches!(det, DetectionResult::BattleStarted) && pending_match_id.is_none() {
                    let m = new_in_progress_match();
                    pending_match_id  = Some(m.id.clone());
                    battle_started_at = Some(Instant::now());
                    log::info!("[capture_loop] battle started (YOLO path) id={}", m.id);
                    let _ = app.emit("battle_started", MatchDetectedPayload {
                        match_data: m, ocr_confidence: 1.0,
                    });
                }
            } else {
                // --- InGame: 冷却後に YOLO でリザルト検知 ---
                let elapsed = battle_started_at.map(|t| t.elapsed().as_secs()).unwrap_or(u64::MAX);
                if elapsed >= RESULT_COOLDOWN_SECS {
                    let dets = match tokio::task::block_in_place(|| yolo.detect(&frame)) {
                        Ok(d)  => d,
                        Err(e) => {
                            log::warn!("[capture_loop] yolo detect: {e}");
                            tokio::time::sleep(interval).await;
                            continue;
                        }
                    };

                    // MyPlayerRow が確信度 0.70 以上で検出されたか確認
                    // → y 中心が PANEL_BOUNDARY_Y_RATIO より上 = WIN パネル内 = 勝ち
                    use crate::detector::PANEL_BOUNDARY_Y_RATIO;
                    let result_opt = YoloDetector::best_detection(&dets, YoloClass::MyPlayerRow)
                        .filter(|d| d.confidence >= 0.70)
                        .map(|d| {
                            let y_center = (d.bbox.y1 + d.bbox.y2) / 2.0;
                            if y_center < PANEL_BOUNDARY_Y_RATIO { "win" } else { "lose" }
                        });

                    if let Some(result_str) = result_opt {
                        match tokio::task::block_in_place(|| extract_from_yolo_detections(&frame, &dets, result_str)) {
                            Ok(data) => {
                                let id = pending_match_id.take();
                                battle_started_at = None;
                                let match_record = new_match_from_ocr(
                                    id, &data.result,
                                    data.kill_count, data.assist_count, data.death_count,
                                    data.xp_after, data.rule, data.stage, data.mode,
                                );
                                log::info!(
                                    "[capture_loop] YOLO result id={} result={} mode={:?} rule={:?} stage={:?}",
                                    match_record.id, match_record.result,
                                    match_record.mode, match_record.rule, match_record.stage,
                                );
                                let _ = app.emit("match_detected", MatchDetectedPayload {
                                    match_data: match_record, ocr_confidence: 1.0,
                                });
                                detector.reset_to_waiting();
                            }
                            Err(e) => log::error!("[capture_loop] YOLO extraction failed: {e}"),
                        }
                    }
                }
            }

        // ══════════════════════════════════════════════════════════════════
        // ピクセルフォールバックパス (YOLO 未配置)
        // ══════════════════════════════════════════════════════════════════
        } else {
            let detection = match tokio::task::block_in_place(|| detector.detect(&frame)) {
                Ok(d)  => d,
                Err(e) => {
                    log::warn!("[capture_loop] pixel detect: {e}");
                    tokio::time::sleep(interval).await;
                    continue;
                }
            };

            match &detection {
                DetectionResult::BattleStarted => {
                    if pending_match_id.is_none() {
                        let m = new_in_progress_match();
                        pending_match_id = Some(m.id.clone());
                        log::info!("[capture_loop] battle started (pixel) id={}", m.id);
                        let _ = app.emit("battle_started", MatchDetectedPayload {
                            match_data: m, ocr_confidence: 1.0,
                        });
                    }
                }
                DetectionResult::Win { arrow_y_ratio } | DetectionResult::Lose { arrow_y_ratio } => {
                    let result_str = detection.result_str().unwrap();
                    match tokio::task::block_in_place(|| extract_match_data(&frame, result_str, *arrow_y_ratio)) {
                        Ok(data) => {
                            let id = pending_match_id.take();
                            let match_record = new_match_from_ocr(
                                id, &data.result,
                                data.kill_count, data.assist_count, data.death_count,
                                data.xp_after, data.rule, data.stage,
                                None, // ピクセルパスではモードを取得しない
                            );
                            log::info!(
                                "[capture_loop] pixel result id={} result={} rule={:?} stage={:?}",
                                match_record.id, match_record.result,
                                match_record.rule, match_record.stage,
                            );
                            let _ = app.emit("match_detected", MatchDetectedPayload {
                                match_data: match_record, ocr_confidence: 1.0,
                            });
                        }
                        Err(e) => log::error!("[capture_loop] pixel extraction failed: {e}"),
                    }
                }
                DetectionResult::NotDetected => {}
            }
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

    tokio::time::sleep(Duration::from_secs(2)).await;
    let in_progress = new_in_progress_match();
    let pending_id  = in_progress.id.clone();
    let _ = app.emit("battle_started", MatchDetectedPayload {
        match_data: in_progress, ocr_confidence: 0.0,
    });

    tokio::time::sleep(Duration::from_secs(3)).await;
    let result = new_match_from_ocr(
        Some(pending_id), "win",
        Some(5), Some(1), Some(2), Some(2341.5),
        Some("ガチエリア".to_string()),
        Some("マテガイ放水路".to_string()),
        Some("Xマッチ".to_string()),
    );
    let _ = app.emit("match_detected", MatchDetectedPayload {
        match_data: result, ocr_confidence: 0.0,
    });

    loop {
        if !*state.is_capturing.lock().await { break; }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
