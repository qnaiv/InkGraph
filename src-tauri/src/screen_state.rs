/// InkGraph — 画面ステートマシン
///
/// 各フレームでどの「画面状態」にいるかを管理する。
/// capture_loop がこのステートマシンを保持し、
/// 検知結果に応じて遷移メソッドを呼び出す。
///
/// 状態遷移図:
///
///   Idle
///     └─(capture_start)──→ WaitingForBattle
///
///   WaitingForBattle
///     └─(battle_start_detected)──→ InGame { match_id }
///
///   InGame
///     └─(result_screen_detected)──→ ResultScreen { match_id }
///     └─(cooldown_expired / no_battle)──→ WaitingForBattle  ← タイムアウト復帰
///
///   ResultScreen
///     └─(extraction_done, x_match)──→ XPowerScreen { match_id }
///     └─(extraction_done, other)───→ WaitingForBattle
///
///   XPowerScreen
///     └─(xp_extracted / timeout)──→ WaitingForBattle

use std::time::Instant;

// ---------------------------------------------------------------------------
// 状態定義
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ScreenState {
    /// キャプチャ停止中 / 初期状態
    Idle,
    /// バトル開始画面を待機中
    WaitingForBattle,
    /// バトル中 — リザルト画面を待機中
    InGame {
        match_id:   String,
        started_at: Instant,
    },
    /// リザルト画面を検知 — OCR で詳細を抽出中
    ResultScreen {
        match_id:    String,
        detected_at: Instant,
    },
    /// Xパワー画面 — Xマッチのリザルト後に遷移 (フェーズ2で使用)
    XPowerScreen {
        match_id:    String,
        detected_at: Instant,
    },
}

impl ScreenState {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Idle              => "Idle",
            Self::WaitingForBattle  => "WaitingForBattle",
            Self::InGame     { .. } => "InGame",
            Self::ResultScreen{ .. }=> "ResultScreen",
            Self::XPowerScreen{ .. }=> "XPowerScreen",
        }
    }
}

// ---------------------------------------------------------------------------
// ステートマシン
// ---------------------------------------------------------------------------

/// InGame タイムアウト: この時間を超えてもリザルトが来なければ WaitingForBattle へ戻す
const IN_GAME_TIMEOUT_SECS: u64 = 600;

/// ResultScreen 滞留タイムアウト: OCR が終わらなくても強制復帰
const RESULT_SCREEN_TIMEOUT_SECS: u64 = 30;

/// XPowerScreen 滞留タイムアウト
const XPOWER_SCREEN_TIMEOUT_SECS: u64 = 30;

pub struct ScreenStateMachine {
    state: ScreenState,
}

impl ScreenStateMachine {
    pub fn new() -> Self {
        Self { state: ScreenState::WaitingForBattle }
    }

    pub fn state(&self) -> &ScreenState { &self.state }

    // ── 遷移メソッド ──────────────────────────────────────────────────────

    /// バトル開始を検知 → InGame へ遷移。
    /// すでに InGame 以降の状態にある場合は何もしない (二重発火防止)。
    pub fn on_battle_started(&mut self, match_id: String) -> bool {
        match &self.state {
            ScreenState::WaitingForBattle => {
                log::info!("[state] WaitingForBattle → InGame (match_id={})", match_id);
                self.state = ScreenState::InGame {
                    match_id,
                    started_at: Instant::now(),
                };
                true
            }
            _ => false,
        }
    }

    /// リザルト画面を検知 → ResultScreen へ遷移。
    /// 現在の match_id を返す。
    pub fn on_result_detected(&mut self) -> Option<String> {
        match &self.state {
            ScreenState::InGame { match_id, .. } => {
                let id = match_id.clone();
                log::info!("[state] InGame → ResultScreen (match_id={})", id);
                self.state = ScreenState::ResultScreen {
                    match_id:    id.clone(),
                    detected_at: Instant::now(),
                };
                Some(id)
            }
            _ => None,
        }
    }

    /// リザルト OCR 完了 (Xマッチ) → XPowerScreen へ遷移 (フェーズ2用)。
    pub fn on_result_extracted_x_match(&mut self) -> Option<String> {
        match &self.state {
            ScreenState::ResultScreen { match_id, .. } => {
                let id = match_id.clone();
                log::info!("[state] ResultScreen → XPowerScreen (match_id={})", id);
                self.state = ScreenState::XPowerScreen {
                    match_id:    id.clone(),
                    detected_at: Instant::now(),
                };
                Some(id)
            }
            _ => None,
        }
    }

    /// 何らかの終了処理が完了 → WaitingForBattle へ戻す。
    pub fn on_done(&mut self) {
        log::info!("[state] {} → WaitingForBattle", self.state.name());
        self.state = ScreenState::WaitingForBattle;
    }

    // ── タイムアウト確認 ───────────────────────────────────────────────────

    /// 各状態のタイムアウトを確認し、超過していれば WaitingForBattle へ戻す。
    /// capture_loop の毎フレームで呼ぶこと。
    pub fn tick_timeouts(&mut self) {
        let should_reset = match &self.state {
            ScreenState::InGame { started_at, match_id } => {
                let elapsed = started_at.elapsed().as_secs();
                if elapsed > IN_GAME_TIMEOUT_SECS {
                    log::warn!("[state] InGame timeout ({elapsed}s) match_id={match_id} → WaitingForBattle");
                    true
                } else { false }
            }
            ScreenState::ResultScreen { detected_at, match_id } => {
                let elapsed = detected_at.elapsed().as_secs();
                if elapsed > RESULT_SCREEN_TIMEOUT_SECS {
                    log::warn!("[state] ResultScreen timeout ({elapsed}s) match_id={match_id} → WaitingForBattle");
                    true
                } else { false }
            }
            ScreenState::XPowerScreen { detected_at, match_id } => {
                let elapsed = detected_at.elapsed().as_secs();
                if elapsed > XPOWER_SCREEN_TIMEOUT_SECS {
                    log::warn!("[state] XPowerScreen timeout ({elapsed}s) match_id={match_id} → WaitingForBattle");
                    true
                } else { false }
            }
            _ => false,
        };

        if should_reset {
            self.state = ScreenState::WaitingForBattle;
        }
    }
}

impl Default for ScreenStateMachine {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = ScreenStateMachine::new();
        assert!(matches!(sm.state(), ScreenState::WaitingForBattle));
    }

    #[test]
    fn test_battle_start_transition() {
        let mut sm = ScreenStateMachine::new();
        assert!(sm.on_battle_started("match-001".to_string()));
        assert!(matches!(sm.state(), ScreenState::InGame { .. }));
    }

    #[test]
    fn test_no_double_battle_start() {
        let mut sm = ScreenStateMachine::new();
        sm.on_battle_started("match-001".to_string());
        // すでに InGame → 二度目は false
        assert!(!sm.on_battle_started("match-002".to_string()));
    }

    #[test]
    fn test_result_detected_transition() {
        let mut sm = ScreenStateMachine::new();
        sm.on_battle_started("match-001".to_string());
        let id = sm.on_result_detected();
        assert_eq!(id.as_deref(), Some("match-001"));
        assert!(matches!(sm.state(), ScreenState::ResultScreen { .. }));
    }

    #[test]
    fn test_result_detected_without_ingame() {
        let mut sm = ScreenStateMachine::new();
        // WaitingForBattle 中にリザルトが来ても遷移しない
        assert!(sm.on_result_detected().is_none());
        assert!(matches!(sm.state(), ScreenState::WaitingForBattle));
    }

    #[test]
    fn test_done_resets_to_waiting() {
        let mut sm = ScreenStateMachine::new();
        sm.on_battle_started("match-001".to_string());
        sm.on_result_detected();
        sm.on_done();
        assert!(matches!(sm.state(), ScreenState::WaitingForBattle));
    }

    #[test]
    fn test_x_match_transition() {
        let mut sm = ScreenStateMachine::new();
        sm.on_battle_started("match-xp".to_string());
        sm.on_result_detected();
        let id = sm.on_result_extracted_x_match();
        assert_eq!(id.as_deref(), Some("match-xp"));
        assert!(matches!(sm.state(), ScreenState::XPowerScreen { .. }));
    }
}
