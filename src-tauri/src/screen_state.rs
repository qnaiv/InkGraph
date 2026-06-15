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
const IN_GAME_TIMEOUT_SECS: u64 = 900;

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
    /// どの状態からでも呼び出せるが、20秒のクールダウンを設ける。
    pub fn on_battle_started(&mut self, match_id: String) -> bool {
        let now = Instant::now();
        let last_event_time = match &self.state {
            ScreenState::Idle => None,
            ScreenState::WaitingForBattle => None,
            ScreenState::InGame { started_at, .. } => Some(*started_at),
            ScreenState::ResultScreen { detected_at, .. } => Some(*detected_at),
            ScreenState::XPowerScreen { detected_at, .. } => Some(*detected_at),
        };

        if let Some(last_time) = last_event_time {
            if now.duration_since(last_time).as_secs() < 20 {
                return false; // クールダウン中
            }
        }

        log::info!("[state] {} -> InGame (match_id={})", self.state.name(), match_id);
        self.state = ScreenState::InGame {
            match_id,
            started_at: now,
        };
        true
    }

    /// リザルト画面を検知。
    /// InGame 中であれば match_id を返し、状態は変更しない。
    pub fn on_result_detected(&mut self) -> Option<String> {
        match &self.state {
            ScreenState::InGame { match_id, .. } => {
                log::debug!("[state] Result detected while InGame (match_id={})", match_id);
                Some(match_id.clone())
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
    fn test_result_detected_returns_id_stays_ingame() {
        let mut sm = ScreenStateMachine::new();
        sm.on_battle_started("match-001".to_string());
        let id = sm.on_result_detected();
        assert_eq!(id.as_deref(), Some("match-001"));
        // on_result_detected は状態遷移しない（InGame のまま）
        assert!(matches!(sm.state(), ScreenState::InGame { .. }));
    }

    #[test]
    fn test_result_detected_without_ingame() {
        let mut sm = ScreenStateMachine::new();
        // WaitingForBattle 中にリザルトが来ても None
        assert!(sm.on_result_detected().is_none());
        assert!(matches!(sm.state(), ScreenState::WaitingForBattle));
    }

    #[test]
    fn test_battle_start_cooldown() {
        let mut sm = ScreenStateMachine::new();
        // 1回目は成功
        assert!(sm.on_battle_started("match-001".to_string()));
        assert!(matches!(sm.state(), ScreenState::InGame { .. }));
        // 20秒以内の再呼び出しはクールダウン中のため失敗
        assert!(!sm.on_battle_started("match-002".to_string()));
        // 状態は変わっていない
        assert!(matches!(sm.state(), ScreenState::InGame { .. }));
    }
}
