// IkaVision XP — ブキ一覧データ (スプラトゥーン3)

export interface Weapon {
  id: string;
  name: string;
  category: WeaponCategory;
  sub: string;
  special: string;
}

export type WeaponCategory =
  | 'シューター'
  | 'マニューバー'
  | 'ブラスター'
  | 'チャージャー'
  | 'スロッシャー'
  | 'スピナー'
  | 'フデ'
  | 'ワイパー'
  | 'シェルター'
  | 'ストリンガー'
  | 'サメライド系';

export const WEAPONS: Weapon[] = [
  // シューター
  { id: 'wakaba', name: 'わかばシューター', category: 'シューター', sub: 'スプラッシュボム', special: 'ウルトラチャクチ' },
  { id: 'splattershot_jr', name: 'スプラシューターコラボ', category: 'シューター', sub: 'キューバンボム', special: 'アメフラシ' },
  { id: 'splattershot', name: 'スプラシューター', category: 'シューター', sub: 'スプラッシュボム', special: 'トリプルトルネード' },
  { id: 'hero_shot', name: 'ヒーローシューターレプリカ', category: 'シューター', sub: 'スプラッシュボム', special: 'トリプルトルネード' },
  { id: 'n_zap_85', name: 'N-ZAP85', category: 'シューター', sub: 'タワーコントロール', special: 'スーパーチャクチ' },
  { id: 'splattershot_pro', name: 'プロモデラーMG', category: 'シューター', sub: 'ポイントセンサー', special: 'キャノン' },
  { id: 'splash_o_matic', name: '52ガロン', category: 'シューター', sub: 'スプラッシュシールド', special: 'ジェットパック' },
  { id: 'aerospray_mg', name: 'ジムワイパー', category: 'シューター', sub: 'スプリンクラー', special: 'スペシャルチャージ' },
  { id: 'jet_squelcher', name: 'ジェットスイーパー', category: 'シューター', sub: 'ポイントセンサー', special: 'スーパーチャクチ' },
  { id: 'l3_nozzlenose', name: 'L3リールガン', category: 'シューター', sub: 'キューバンボム', special: 'ウルトラチャクチ' },
  { id: 'h3_nozzlenose', name: 'H3リールガン', category: 'シューター', sub: 'ポイントセンサー', special: 'キャノン' },
  { id: 'squeezer', name: 'ボトルガイザー', category: 'シューター', sub: 'スプラッシュボム', special: 'グレートバリア' },

  // マニューバー
  { id: 'splat_dualies', name: 'スプラマニューバー', category: 'マニューバー', sub: 'ロボットボム', special: 'ジェットパック' },
  { id: 'dapple_dualies', name: 'スパッタリー', category: 'マニューバー', sub: 'スプリンクラー', special: 'スーパーチャクチ' },
  { id: 'glooga_dualies', name: 'デュアルスイーパー', category: 'マニューバー', sub: 'スプラッシュシールド', special: 'カニタンク' },
  { id: 'enperry_dualies', name: 'ケルビン525', category: 'マニューバー', sub: 'キューバンボム', special: 'アメフラシ' },
  { id: 'tetra_dualies', name: 'Dualieスクウィークス', category: 'マニューバー', sub: 'スプラッシュボム', special: 'ウルトラチャクチ' },
  { id: 'dark_tetra_dualies', name: 'クアッドホッパーブラック', category: 'マニューバー', sub: 'トーピード', special: 'マルチミサイル' },

  // ブラスター
  { id: 'blaster', name: 'ノヴァブラスター', category: 'ブラスター', sub: 'ポイントセンサー', special: 'アメフラシ' },
  { id: 'clash_blaster', name: 'クラッシュブラスター', category: 'ブラスター', sub: 'スプラッシュボム', special: 'カニタンク' },
  { id: 'range_blaster', name: 'ロングブラスター', category: 'ブラスター', sub: 'スプラッシュシールド', special: 'グレートバリア' },
  { id: 'rapid_blaster', name: 'ラピッドブラスター', category: 'ブラスター', sub: 'フィッシュフライ', special: 'ウルトラハンコ' },
  { id: 'rapid_blaster_pro', name: 'Rブラスターエリート', category: 'ブラスター', sub: 'ポイントセンサー', special: 'ジェットパック' },

  // チャージャー
  { id: 'squiffer', name: 'スクイックリンα', category: 'チャージャー', sub: 'ポイントセンサー', special: 'スーパーチャクチ' },
  { id: 'splat_charger', name: 'スプラチャージャー', category: 'チャージャー', sub: 'スプラッシュボム', special: 'マルチミサイル' },
  { id: 'splatterscope', name: 'スプラスコープ', category: 'チャージャー', sub: 'スプラッシュボム', special: 'マルチミサイル' },
  { id: 'e_liter_4k', name: 'リッター4K', category: 'チャージャー', sub: 'ポイントセンサー', special: 'ウルトラチャクチ' },
  { id: 'e_liter_4k_scope', name: '4Kスコープ', category: 'チャージャー', sub: 'ポイントセンサー', special: 'ウルトラチャクチ' },
  { id: 'bamboozler_14_mk1', name: 'バケットスロッシャー', category: 'チャージャー', sub: 'スプリンクラー', special: 'スーパーチャクチ' },
  { id: 'goo_tuber', name: 'クーゲルシュライバー', category: 'チャージャー', sub: 'トーピード', special: 'グレートバリア' },

  // スロッシャー
  { id: 'slosher', name: 'バケットスロッシャー', category: 'スロッシャー', sub: 'スプラッシュボム', special: 'ホップソナー' },
  { id: 'tri_slosher', name: 'ヒッセン', category: 'スロッシャー', sub: 'スプラッシュボム', special: 'ウルトラチャクチ' },
  { id: 'sloshing_machine', name: 'スクリュースロッシャー', category: 'スロッシャー', sub: 'スプリンクラー', special: 'キャノン' },
  { id: 'bloblobber', name: 'オーバーフロッシャー', category: 'スロッシャー', sub: 'フィッシュフライ', special: 'ウルトラハンコ' },
  { id: 'explosher', name: 'エクスプロッシャー', category: 'スロッシャー', sub: 'ポイントセンサー', special: 'カニタンク' },

  // スピナー
  { id: 'mini_splatling', name: 'スプラスピナー', category: 'スピナー', sub: 'スプラッシュボム', special: 'ジェットパック' },
  { id: 'heavy_splatling', name: 'バレルスピナー', category: 'スピナー', sub: 'スプラッシュシールド', special: 'マルチミサイル' },
  { id: 'hydra_splatling', name: 'ハイドラント', category: 'スピナー', sub: 'スプリンクラー', special: 'グレートバリア' },
  { id: 'ballpoint_splatling', name: 'クーゲルシュライバー', category: 'スピナー', sub: 'スプラッシュボム', special: 'カニタンク' },
  { id: 'nautilus_47', name: 'ノーチラス47', category: 'スピナー', sub: 'ポイントセンサー', special: 'スーパーチャクチ' },

  // フデ
  { id: 'inkbrush', name: 'パブロ', category: 'フデ', sub: 'スプリンクラー', special: 'ウルトラチャクチ' },
  { id: 'octobrush', name: 'ホクサイ', category: 'フデ', sub: 'スプラッシュボム', special: 'ウルトラチャクチ' },

  // ワイパー
  { id: 'splatana_wiper', name: 'ジムワイパー', category: 'ワイパー', sub: 'フィッシュフライ', special: 'キャノン' },
  { id: 'splatana_stamper', name: 'デンタルワイパーミント', category: 'ワイパー', sub: 'トーピード', special: 'トリプルトルネード' },

  // シェルター
  { id: 'splat_brella', name: 'パラシェルター', category: 'シェルター', sub: 'スプラッシュボム', special: 'ウルトラハンコ' },
  { id: 'tenta_brella', name: 'キャンピングシェルター', category: 'シェルター', sub: 'スプリンクラー', special: 'カニタンク' },
  { id: 'undercover_brella', name: 'スパイガジェット', category: 'シェルター', sub: 'スプラッシュボム', special: 'グレートバリア' },

  // ストリンガー
  { id: 'tri_stringer', name: 'トライストリンガー', category: 'ストリンガー', sub: 'タワーコントロール', special: 'ホップソナー' },
  { id: 'inkbow', name: 'LACT-450', category: 'ストリンガー', sub: 'ロボットボム', special: 'カニタンク' },
];

/** 最近使ったブキを localStorage から取得 */
export function getRecentWeapons(n = 5): Weapon[] {
  try {
    const raw = localStorage.getItem('ikavision_recent_weapons');
    if (!raw) return [];
    const ids: string[] = JSON.parse(raw);
    return ids
      .slice(0, n)
      .map((id) => WEAPONS.find((w) => w.id === id))
      .filter((w): w is Weapon => w != null);
  } catch {
    return [];
  }
}

/** 最近使ったブキを更新 */
export function pushRecentWeapon(weaponId: string): void {
  try {
    const raw = localStorage.getItem('ikavision_recent_weapons');
    const ids: string[] = raw ? JSON.parse(raw) : [];
    const updated = [weaponId, ...ids.filter((id) => id !== weaponId)].slice(0, 10);
    localStorage.setItem('ikavision_recent_weapons', JSON.stringify(updated));
  } catch {
    // ignore
  }
}
