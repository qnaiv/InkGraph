// InkGraph — フロントエンド型定義

export interface Match {
  id: string;
  played_at: string;
  mode: string | null;  // "Xマッチ" / "バンカラマッチ(チャレンジ)" / "ナワバリバトル" 等
  rule: string | null;
  stage: string | null;
  weapon: string | null;
  result: 'win' | 'lose' | 'in_progress';
  kill_count: number | null;
  assist_count: number | null;
  death_count: number | null;
  xp_after: number | null;
  tags: string[]; // フロントエンドでは配列として扱う
  note: string | null;
  created_at?: string;
  updated_at?: string;
}

export interface MatchDetectedPayload {
  match_data: RawMatch;
  ocr_confidence: number;
}

/** Rust から届く生データ (tags が JSON 文字列) */
export interface RawMatch extends Omit<Match, 'tags'> {
  tags: string | null; // JSON 配列文字列
}

export interface WindowInfo {
  hwnd: number;
  title: string;
}

export interface OcrTestResult {
  raw_text: string;
  lines: string[];
}

export interface CaptureStatusPayload {
  active: boolean;
  fps: number;
  window_title: string | null;
}

export interface CaptureDebugResult {
  frame_w: number;
  frame_h: number;
  battle_start_text: string;
  battle_start_found: boolean;
  dark_scroll_found: boolean;
  win_grey_rows: number;
  lose_grey_rows: number;
  rule_ocr_text: string;
  rule_normalized: string | null;
  stage_ocr_text: string;
  stage_normalized: string | null;
  win_roi_text: string;
  win_text_found: boolean;
  yellow_win_px: number;
  yellow_lose_px: number;
  centroid_y: number;
  y_spread: number;
  detection_summary: string;
}

export interface XpDataPoint {
  played_at: string;
  xp_after: number;
  result: 'win' | 'lose' | 'in_progress';
}

// ルール一覧
export const RULES = ['ガチエリア', 'ガチヤグラ', 'ガチホコ', 'ガチアサリ'] as const;
export type Rule = (typeof RULES)[number];

// 定型反省タグ
export const PRESET_TAGS = [
  '初動デス',
  '打開成功',
  '打開失敗',
  '打開できず',
  'カウント負け',
  'エナジースタン',
  '正面勝負',
  '連デス',
  '潜伏ミス',
  '塗りサボり',
  '良い立ち回り',
  'スペシャル活かせた',
  'スペシャル空撃ち',
] as const;
