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
        cascade::StatsDetector,
        db::{new_in_progress_match, new_match_from_ocr},
        detector::{
            pixel_result_check, YoloClass, YoloDetector, DEFAULT_CONF_THRESHOLD, PANEL_BOUNDARY_Y,
        },
        extractor::extract_from_yolo_detections,
        screen_state::{ScreenState, ScreenStateMachine},
        types::MatchDetectedPayload,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // ── Detector 初期化 ────────────────────────────────────────────────
    let model_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("assets/models/yolo_result.onnx")))
        .unwrap_or_else(|| std::path::PathBuf::from("assets/models/yolo_result.onnx"));

    let yolo = Arc::new(Mutex::new(YoloDetector::new(&model_path)));
    {
        let mut yolo_guard = yolo.lock().await;
        if let Err(e) = tokio::task::block_in_place(|| yolo_guard.load()) {
            log::error!("[capture_loop] YOLO load failed — キャプチャを中断します: {e}");
            let _ = app.emit("capture_status", CaptureStatusPayload {
                active: false,
                fps: 0.0,
                window_title: None,
                yolo_loaded: false,
            });
            return;
        }
        log::info!("[capture_loop] YOLO loaded: {}", model_path.display());
    }

    {
        let rec_path = model_path.with_file_name("ppocr_rec_ja.onnx");
        let dict_path = model_path.with_file_name("ppocr_dict_ja.txt");
        if let Err(e) = tokio::task::block_in_place(|| crate::ocr_rec::init(&rec_path, &dict_path))
        {
            log::warn!("[capture_loop] ocr_rec init failed: {e}");
        }
    }

    let stats_model_path = model_path.with_file_name("yolo_stats.onnx");
    let stats = Arc::new(Mutex::new(StatsDetector::new(&stats_model_path)));
    {
        let mut stats_guard = stats.lock().await;
        if let Err(e) = tokio::task::block_in_place(|| stats_guard.load()) {
            log::warn!("[capture_loop] Model 2 load failed (OCR fallback only): {e}");
        }
    }

    // ── WGC セッション作成 ─────────────────────────────────────────────────
    let session = match tokio::task::block_in_place(|| WindowCaptureSession::new(hwnd)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[capture_loop] WGC session failed: {e}");
            let _ = app.emit("capture_status", CaptureStatusPayload {
                active: false,
                fps: 0.0,
                window_title: None,
                yolo_loaded: false,
            });
            return;
        }
    };

    {
        let yolo_guard = yolo.lock().await;
        let _ = app.emit("capture_status", CaptureStatusPayload {
            active: true,
            fps: 5.0,
            window_title: None,
            yolo_loaded: yolo_guard.is_loaded(),
        });
    }

    let interval = Duration::from_millis(200); // 5 fps
    let mut frame_count: u64 = 0;
    let mut state_machine = ScreenStateMachine::new();

    loop {
        if !*state.is_capturing.lock().await {
            break;
        }

        state_machine.tick_timeouts();

        let frame = match tokio::task::block_in_place(|| session.get_frame()) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("[capture_loop] get_frame: {e}");
                tokio::time::sleep(interval).await;
                continue;
            }
        };
        *state.last_frame.lock().await = Some(frame.clone());

        frame_count += 1;
        if frame_count % 5 == 0 {
            // 1秒に1回、現在の状態をログ出力
            log::info!("[capture_loop] state: {}", state_machine.state().name());
        }

        // ── YOLO 検知 ────────────────────────────────────────────────────────
        let dets = {
            let mut yolo_guard = yolo.lock().await;
            match tokio::task::block_in_place(|| yolo_guard.detect(&frame)) {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("[capture_loop] yolo detect: {e}");
                    tokio::time::sleep(interval).await;
                    continue;
                }
            }
        };

        // ── 1. バトル開始を常に検知 ──────────────────────────────────────
        if let Some(bs) = YoloDetector::best_detection(&dets, YoloClass::BattleStart) {
            if bs.confidence >= DEFAULT_CONF_THRESHOLD {
                let m = new_in_progress_match();
                if state_machine.on_battle_started(m.id.clone()) {
                    log::info!("[capture_loop] battle started (YOLO BattleStart) id={}", m.id);
                    let _ = app.emit(
                        "battle_started",
                        MatchDetectedPayload {
                            match_data: m,
                            ocr_confidence: 1.0,
                        },
                    );
                } else {
                    log::warn!("[capture_loop] on_battle_started returned false. Cooldown active?");
                }
            } else {
                log::info!(
                    "[capture_loop] BattleStart candidate found with confidence: {:.2} (threshold: {})",
                    bs.confidence,
                    DEFAULT_CONF_THRESHOLD
                );
            }
        }

        // ── 2. リザルト画面を常に検知 ────────────────────────────────────
        let win_conf = YoloDetector::best_detection(&dets, YoloClass::Win)
            .map(|d| d.confidence)
            .unwrap_or(0.0);
        let lose_conf = YoloDetector::best_detection(&dets, YoloClass::Lose)
            .map(|d| d.confidence)
            .unwrap_or(0.0);
        let draw_conf = YoloDetector::best_detection(&dets, YoloClass::Draw)
            .map(|d| d.confidence)
            .unwrap_or(0.0);
        let arrow_conf = YoloDetector::best_detection(&dets, YoloClass::MyArrow)
            .map(|d| d.confidence)
            .unwrap_or(0.0);
        let is_result_screen = win_conf >= 0.30 || lose_conf >= 0.30 || arrow_conf >= 0.40;

        let result_opt = if draw_conf >= 0.55 {
            Some("draw")
        } else if is_result_screen {
            let arrow_y = YoloDetector::best_detection(&dets, YoloClass::MyArrow)
                .map(|d| (d.bbox.y1 + d.bbox.y2) / 2.0);
            log::info!("[capture_loop] result screen, MyArrow y={:?}", arrow_y);
            match arrow_y {
                Some(y) if y < PANEL_BOUNDARY_Y => Some("win"),
                Some(_) => Some("lose"),
                None => pixel_result_check(&frame),
            }
        } else {
            let px = pixel_result_check(&frame);
            if px.is_some() {
                log::info!("[capture_loop] YOLO miss → pixel fallback: {:?}", px);
            }
            px
        };

        if let Some(result_str) = result_opt {
            if let Some(id) = state_machine.on_result_detected() {
                log::info!("[capture_loop] result detected, starting async extraction for id={id}");

                // ── 非同期抽出タスク ──────────────────────────────────
                let app_handle = app.clone();
                let frame_clone = frame.clone();
                let dets_clone = dets.clone();
                let result_clone = result_str.to_string();
                let stats_clone = Arc::clone(&stats);
                let arrow_for_stats =
                    YoloDetector::best_detection(&dets, YoloClass::MyArrow).cloned();

                tokio::spawn(async move {
                    let (stats_override, header_override, crop_base64, crop_header_base64) = {
                        let mut stats_guard = stats_clone.lock().await;
                        if stats_guard.is_loaded() {
                            tokio::task::block_in_place(|| {
                                let stats_ov =
                                    arrow_for_stats.as_ref().and_then(|a| {
                                        stats_guard.run_cascade(&frame_clone, a).ok()
                                    });
                                let header_ov =
                                    stats_guard.run_header_cascade(&frame_clone).ok();
                                let crop = arrow_for_stats.as_ref().and_then(|a| {
                                    stats_guard.get_stats_crop_base64(&frame_clone, a)
                                });
                                let crop_header =
                                    stats_guard.get_header_crop_base64(&frame_clone);
                                (stats_ov, header_ov, crop, crop_header)
                            })
                        } else {
                            (None, None, None, None)
                        }
                    };

                    match tokio::task::block_in_place(|| {
                        extract_from_yolo_detections(
                            &frame_clone,
                            &dets_clone,
                            &result_clone,
                            stats_override,
                            header_override,
                        )
                    }) {
                        Ok(data) => {
                            let match_record = new_match_from_ocr(
                                Some(id.clone()),
                                &data.result,
                                data.kill_count,
                                data.death_count,
                                data.special_count,
                                data.paint_count,
                                data.xp_after,
                                data.rule,
                                data.stage,
                                data.mode,
                                data.gold_award_count,
                                crop_base64,
                                crop_header_base64,
                            );
                            log::info!(
                                "[capture_loop:async] YOLO result id={} result={} mode={:?} rule={:?} stage={:?}",
                                match_record.id, match_record.result,
                                match_record.mode, match_record.rule, match_record.stage,
                            );
                            let _ = app_handle.emit(
                                "match_detected",
                                MatchDetectedPayload {
                                    match_data: match_record,
                                    ocr_confidence: 1.0,
                                },
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "[capture_loop:async] YOLO extraction failed (id={id}): {e}"
                            );
                        }
                    }
                });
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
        Some(5), Some(1), Some(2),
        None, // paint_count: スタブは None
        Some(2341.5),
        Some("ガチエリア".to_string()),
        Some("マテガイ放水路".to_string()),
        Some("Xマッチ".to_string()),
        Some(1), // スタブ: 金表彰1枚
        None, None,
    );
    let _ = app.emit("match_detected", MatchDetectedPayload {
        match_data: result, ocr_confidence: 0.0,
    });

    loop {
        if !*state.is_capturing.lock().await { break; }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
