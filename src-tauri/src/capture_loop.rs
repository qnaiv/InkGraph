/// InkGraph — キャプチャループ (YOLO 専用)
///
/// yolo_result.onnx が未配置の場合はエラーで終了する。
/// バトル開始: YOLO BattleStart クラスで検知
/// リザルト検知: Win/Lose バナー検知 → MyArrow Y 座標で勝敗判定 → BBox OCR

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
        active: false, fps: 0.0, window_title: None, yolo_loaded: false,
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
        detector::{pixel_result_check, YoloClass, YoloDetector, PANEL_BOUNDARY_Y},
        extractor::extract_from_yolo_detections,
        types::MatchDetectedPayload,
    };

    // ── YOLO モデルをロード (失敗したらエラーで中断) ─────────────────────
    let model_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("assets/models/yolo_result.onnx")))
        .unwrap_or_else(|| std::path::PathBuf::from("assets/models/yolo_result.onnx"));

    let mut yolo = YoloDetector::new(&model_path);
    if let Err(e) = tokio::task::block_in_place(|| yolo.load()) {
        log::error!("[capture_loop] YOLO load failed — キャプチャを中断します: {e}");
        let _ = app.emit("capture_status", CaptureStatusPayload {
            active: false, fps: 0.0, window_title: None, yolo_loaded: false,
        });
        return;
    }
    log::info!("[capture_loop] YOLO loaded: {}", model_path.display());

    // ── WGC セッション作成 ─────────────────────────────────────────────────
    let session = match tokio::task::block_in_place(|| WindowCaptureSession::new(hwnd)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[capture_loop] WGC session failed: {e}");
            let _ = app.emit("capture_status", CaptureStatusPayload {
                active: false, fps: 0.0, window_title: None, yolo_loaded: false,
            });
            return;
        }
    };

    let _ = app.emit("capture_status", CaptureStatusPayload {
        active: true, fps: 5.0, window_title: None, yolo_loaded: yolo.is_loaded(),
    });

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

        // ── YOLO 検知 ────────────────────────────────────────────────────────
        {
            let dets = match tokio::task::block_in_place(|| yolo.detect(&frame)) {
                Ok(d)  => d,
                Err(e) => {
                    log::warn!("[capture_loop] yolo detect: {e}");
                    tokio::time::sleep(interval).await;
                    continue;
                }
            };

            if pending_match_id.is_none() {
                // --- BattleStart クラスで試合開始を検知 ---
                if YoloDetector::best_detection(&dets, YoloClass::BattleStart)
                    .filter(|d| d.confidence >= 0.60)
                    .is_some()
                {
                    let m = new_in_progress_match();
                    pending_match_id  = Some(m.id.clone());
                    battle_started_at = Some(Instant::now());
                    log::info!("[capture_loop] battle started (YOLO BattleStart) id={}", m.id);
                    let _ = app.emit("battle_started", MatchDetectedPayload {
                        match_data: m, ocr_confidence: 1.0,
                    });
                }
            } else {
                // --- 冷却後に Win / Lose / Draw クラスでリザルト検知 ---
                let elapsed = battle_started_at.map(|t| t.elapsed().as_secs()).unwrap_or(u64::MAX);
                if elapsed >= RESULT_COOLDOWN_SECS {
                    // Win/Lose クラスはリザルト画面のバナーテキストを検知するため
                    // 負け画面でも Win が高信頼度で出る。どちらかが 0.30 以上なら
                    // 「リザルト画面にいる」とみなし、MyArrow の Y 座標で勝敗判定。
                    let win_conf  = YoloDetector::best_detection(&dets, YoloClass::Win).map(|d| d.confidence).unwrap_or(0.0);
                    let lose_conf = YoloDetector::best_detection(&dets, YoloClass::Lose).map(|d| d.confidence).unwrap_or(0.0);
                    let draw_conf = YoloDetector::best_detection(&dets, YoloClass::Draw).map(|d| d.confidence).unwrap_or(0.0);
                    let is_result_screen = win_conf >= 0.30 || lose_conf >= 0.30;
                    log::debug!(
                        "[capture_loop] result candidates: win={win_conf:.2} lose={lose_conf:.2} draw={draw_conf:.2} result_screen={is_result_screen}"
                    );
                    let result_opt = if draw_conf >= 0.55 {
                        Some("draw")
                    } else if is_result_screen {
                        // MyArrow Y 中心 < PANEL_BOUNDARY_Y (0.630) → WIN 側、>= → LOSE 側
                        let arrow_y = YoloDetector::best_detection(&dets, YoloClass::MyArrow)
                            .map(|d| (d.bbox.y1 + d.bbox.y2) / 2.0);
                        log::info!("[capture_loop] result screen, MyArrow y={:?}", arrow_y);
                        match arrow_y {
                            Some(y) if y < PANEL_BOUNDARY_Y => Some("win"),
                            Some(_)                          => Some("lose"),
                            None                             => pixel_result_check(&frame),
                        }
                    } else {
                        // YOLO がリザルト画面を認識できなかった場合はピクセル判定
                        let px = pixel_result_check(&frame);
                        if px.is_some() {
                            log::info!("[capture_loop] YOLO miss → pixel fallback: {:?}", px);
                        }
                        px
                    };

                    if let Some(result_str) = result_opt {
                        match tokio::task::block_in_place(|| extract_from_yolo_detections(&frame, &dets, result_str)) {
                            Ok(data) => {
                                let id = pending_match_id.take();
                                battle_started_at = None;
                                let match_record = new_match_from_ocr(
                                    id, &data.result,
                                    data.kill_count, data.death_count, data.special_count,
                                    data.xp_after, data.rule, data.stage, data.mode,
                                    data.gold_award_count,
                                );
                                log::info!(
                                    "[capture_loop] YOLO result id={} result={} mode={:?} rule={:?} stage={:?}",
                                    match_record.id, match_record.result,
                                    match_record.mode, match_record.rule, match_record.stage,
                                );
                                let _ = app.emit("match_detected", MatchDetectedPayload {
                                    match_data: match_record, ocr_confidence: 1.0,
                                });
                            }
                            Err(e) => log::error!("[capture_loop] YOLO extraction failed: {e}"),
                        }
                    }
                }
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
        active: true, fps: 0.0, window_title: None, yolo_loaded: false,
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
        Some(1), // スタブ: 金表彰1枚
    );
    let _ = app.emit("match_detected", MatchDetectedPayload {
        match_data: result, ocr_confidence: 0.0,
    });

    loop {
        if !*state.is_capturing.lock().await { break; }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
