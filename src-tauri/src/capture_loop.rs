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
        detector::{pixel_result_check, YoloClass, YoloDetector, PANEL_BOUNDARY_Y},
        extractor::extract_from_yolo_detections,
        types::MatchDetectedPayload,
    };

    // ── Model 1: YOLO モデルをロード (失敗したらエラーで中断) ───────────
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

    // ── Model 2: スタッツモデルをロード (失敗しても続行 — オプション) ───
    let stats_model_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("assets/models/yolo_stats.onnx")))
        .unwrap_or_else(|| std::path::PathBuf::from("assets/models/yolo_stats.onnx"));
    let mut stats_detector = StatsDetector::new(&stats_model_path);
    match tokio::task::block_in_place(|| stats_detector.load()) {
        Ok(_)  => log::info!("[capture_loop] stats model loaded: {}", stats_model_path.display()),
        Err(e) => log::warn!("[capture_loop] yolo_stats not loaded (cascade disabled): {e}"),
    }

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
    // リザルト未検知のまま放置された試合を強制リセットするタイムアウト。
    // 通常のXマッチは最長でも10分程度のため、30分経過したら固着とみなす。
    const STUCK_MATCH_TIMEOUT_SECS: u64 = 30 * 60;

    loop {
        if !*state.is_capturing.lock().await { break; }

        // ── 固着チェック: pending_match_id が長時間クリアされなかった場合はリセット ─
        if pending_match_id.is_some() {
            let elapsed = battle_started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            if elapsed >= STUCK_MATCH_TIMEOUT_SECS {
                log::warn!(
                    "[capture_loop] match stuck for {elapsed}s — resetting pending_match_id"
                );
                pending_match_id  = None;
                battle_started_at = None;
            }
        }

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
                // pending_match_id が設定中に BattleStart が再検知された場合はデバッグログ
                if let Some(bs) = YoloDetector::best_detection(&dets, YoloClass::BattleStart) {
                    if bs.confidence >= 0.60 {
                        let elapsed = battle_started_at
                            .map(|t| t.elapsed().as_secs())
                            .unwrap_or(0);
                        log::warn!(
                            "[capture_loop] BattleStart detected (conf={:.2}) but already tracking \
                             id={:?} (elapsed={elapsed}s) — skipping",
                            bs.confidence, pending_match_id
                        );
                    }
                }

                // --- 冷却後に Win / Lose / Draw クラスでリザルト検知 ---
                let elapsed = battle_started_at.map(|t| t.elapsed().as_secs()).unwrap_or(u64::MAX);
                if elapsed >= RESULT_COOLDOWN_SECS {
                    // Win/Lose クラスはリザルト画面のバナーテキストを検知するため
                    // 負け画面でも Win が高信頼度で出る。どちらかが 0.30 以上なら
                    // 「リザルト画面にいる」とみなし、MyArrow の Y 座標で勝敗判定。
                    let win_conf  = YoloDetector::best_detection(&dets, YoloClass::Win).map(|d| d.confidence).unwrap_or(0.0);
                    let lose_conf = YoloDetector::best_detection(&dets, YoloClass::Lose).map(|d| d.confidence).unwrap_or(0.0);
                    let draw_conf = YoloDetector::best_detection(&dets, YoloClass::Draw).map(|d| d.confidence).unwrap_or(0.0);
                    let arrow_conf = YoloDetector::best_detection(&dets, YoloClass::MyArrow).map(|d| d.confidence).unwrap_or(0.0);
                    // Win/Lose バナー未検知でも MyArrow が見えていればリザルト画面とみなす
                    let is_result_screen = win_conf >= 0.30 || lose_conf >= 0.30 || arrow_conf >= 0.40;
                    log::debug!(
                        "[capture_loop] result candidates: win={win_conf:.2} lose={lose_conf:.2} draw={draw_conf:.2} arrow={arrow_conf:.2} result_screen={is_result_screen}"
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
                        // カスケード推論 (Model 2): MyArrow が検出されていれば実行
                        let cascade_stats = YoloDetector::best_detection(&dets, YoloClass::MyArrow)
                            .and_then(|arrow| {
                                tokio::task::block_in_place(|| {
                                    stats_detector.run_cascade(&frame, arrow).ok()
                                })
                            });

                        // ヘッダー部 YOLO 推論: モード/ルール/ステージをクラス検出で取得
                        let header_info = tokio::task::block_in_place(|| {
                            stats_detector.run_header_cascade(&frame).ok()
                        });

                        match tokio::task::block_in_place(|| extract_from_yolo_detections(&frame, &dets, result_str, cascade_stats, header_info)) {
                            Ok(data) => {
                                let id = pending_match_id.take();
                                battle_started_at = None;
                                let match_record = new_match_from_ocr(
                                    id, &data.result,
                                    data.kill_count, data.death_count, data.special_count,
                                    data.paint_count,
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
                            Err(e) => {
                                // 抽出に失敗した場合も pending_match_id をリセットする。
                                // リセットしないと次の BattleStart 検知 (L139) が
                                // ブロックされ続け、固着タイムアウト (30分) まで
                                // 以降の試合が一切記録されなくなってしまう。
                                log::error!(
                                    "[capture_loop] YOLO extraction failed (id={:?}): {e} — resetting pending_match_id",
                                    pending_match_id
                                );
                                pending_match_id  = None;
                                battle_started_at = None;
                            }
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
        Some(5), Some(1), Some(2),
        None, // paint_count: スタブは None
        Some(2341.5),
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
