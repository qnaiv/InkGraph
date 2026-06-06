// InkGraph — ブキ一覧データ (スプラトゥーン3)

export interface Weapon {
  id: string;
  name: string;
  category: WeaponCategory;
  sub: string;
  special: string;
}

export type WeaponCategory =
  | 'シューター'
  | 'ローラー'
  | 'マニューバー'
  | 'ブラスター'
  | 'チャージャー'
  | 'スロッシャー'
  | 'スピナー'
  | 'フデ'
  | 'ワイパー'
  | 'シェルター'
  | 'ストリンガー';

export const WEAPONS: Weapon[] = [
  // シューター
  { id: 'wakaba', name: 'わかばシューター', category: 'シューター', sub: 'スプラッシュボム', special: 'グレートバリア' },
  { id: 'momiji', name: 'もみじシューター', category: 'シューター', sub: 'トラップ', special: 'ナイスダマ' },
  { id: 'splat_shooter', name: 'スプラシューター', category: 'シューター', sub: 'スプラッシュボム', special: 'ウルトラショット' },
  { id: 'splat_shooter_collabo', name: 'スプラシューターコラボ', category: 'シューター', sub: 'キューバンボム', special: 'アメフラシ' },
  { id: 'n_zap_85', name: 'N-ZAP85', category: 'シューター', sub: 'スプラッシュボム', special: 'ショクワンダー' },
  { id: 'gallon_52', name: '.52ガロン', category: 'シューター', sub: 'カーリングボム', special: 'ウルトラチャクチ' },
  { id: 'gallon_96', name: '.96ガロン', category: 'シューター', sub: 'タンサンボム', special: 'メガホンレーザー5.1ch' },
  { id: 'promodeler_mg', name: 'プロモデラーMG', category: 'シューター', sub: 'ポイントセンサー', special: 'キューインキ' },
  { id: 'jet_sweeper', name: 'ジェットスイーパー', category: 'シューター', sub: 'ポイントセンサー', special: 'ジェットパック' },
  { id: 'l3_reelgun', name: 'L3リールガン', category: 'シューター', sub: 'キューバンボム', special: 'ウルトラチャクチ' },
  { id: 'h3_reelgun', name: 'H3リールガン', category: 'シューター', sub: 'ポイントセンサー', special: 'メガホンレーザー5.1ch' },
  { id: 'prime_shooter', name: 'プライムシューター', category: 'シューター', sub: 'トーピード', special: 'ウルトラショット' },
  { id: 'bottle_geyser', name: 'ボトルガイザー', category: 'シューター', sub: 'スプラッシュボム', special: 'グレートバリア' },
  { id: 'space_shooter', name: 'スペースシューター', category: 'シューター', sub: 'ロボットボム', special: 'ナイスダマ' },
  { id: 'hero_shooter_replica', name: 'ヒーローシューターレプリカ', category: 'シューター', sub: 'スプラッシュボム', special: 'トリプルトルネード' },
  { id: 'octo_shooter_replica', name: 'オクタシューターレプリカ', category: 'シューター', sub: 'ポイズンミスト', special: 'カニタンク' },

  // ローラー
  { id: 'splat_roller', name: 'スプラローラー', category: 'ローラー', sub: 'カーリングボム', special: 'ナイスダマ' },
  { id: 'carbon_roller', name: 'カーボンローラー', category: 'ローラー', sub: 'スプリンクラー', special: 'ジェットパック' },
  { id: 'dynamo_roller', name: 'ダイナモローラー', category: 'ローラー', sub: 'トラップ', special: 'メガホンレーザー5.1ch' },
  { id: 'variable_roller', name: 'ヴァリアブルローラー', category: 'ローラー', sub: 'ポイントセンサー', special: 'ホップソナー' },
  { id: 'wide_roller', name: 'ワイドローラー', category: 'ローラー', sub: 'ポイズンミスト', special: 'ウルトラチャクチ' },

  // ブラスター
  { id: 'nova_blaster', name: 'ノヴァブラスター', category: 'ブラスター', sub: 'ポイントセンサー', special: 'アメフラシ' },
  { id: 'clash_blaster', name: 'クラッシュブラスター', category: 'ブラスター', sub: 'スプラッシュボム', special: 'カニタンク' },
  { id: 'long_blaster', name: 'ロングブラスター', category: 'ブラスター', sub: 'スプラッシュシールド', special: 'グレートバリア' },
  { id: 'rapid_blaster', name: 'ラピッドブラスター', category: 'ブラスター', sub: 'トーピード', special: 'ナイスダマ' },
  { id: 'r_blaster_elite', name: 'Rブラスターエリート', category: 'ブラスター', sub: 'ポイントセンサー', special: 'ジェットパック' },

  // チャージャー
  { id: 'splat_charger', name: 'スプラチャージャー', category: 'チャージャー', sub: 'スプラッシュボム', special: 'マルチミサイル' },
  { id: 'splat_scope', name: 'スプラスコープ', category: 'チャージャー', sub: 'スプラッシュボム', special: 'マルチミサイル' },
  { id: 'squiclean_a', name: 'スクイックリンα', category: 'チャージャー', sub: 'ポイントセンサー', special: 'スーパーチャクチ' },
  { id: 'liter_4k', name: 'リッター4K', category: 'チャージャー', sub: 'ポイントセンサー', special: 'ウルトラチャクチ' },
  { id: 'liter_4k_scope', name: '4Kスコープ', category: 'チャージャー', sub: 'ポイントセンサー', special: 'ウルトラチャクチ' },

  // スロッシャー
  { id: 'bucket_slosher', name: 'バケットスロッシャー', category: 'スロッシャー', sub: 'スプラッシュボム', special: 'ホップソナー' },
  { id: 'hissen', name: 'ヒッセン', category: 'スロッシャー', sub: 'スプラッシュボム', special: 'ウルトラチャクチ' },
  { id: 'screw_slosher', name: 'スクリュースロッシャー', category: 'スロッシャー', sub: 'スプリンクラー', special: 'カニタンク' },
  { id: 'over_flosher', name: 'オーバーフロッシャー', category: 'スロッシャー', sub: 'タンサンボム', special: 'デコイチラシ' },
  { id: 'explosher', name: 'エクスプロッシャー', category: 'スロッシャー', sub: 'ポイントセンサー', special: 'カニタンク' },
  { id: 'mopurin', name: 'モップリン', category: 'スロッシャー', sub: 'ジャンプビーコン', special: 'ショクワンダー' },

  // スピナー
  { id: 'splat_spinner', name: 'スプラスピナー', category: 'スピナー', sub: 'スプラッシュボム', special: 'ジェットパック' },
  { id: 'barrel_spinner', name: 'バレルスピナー', category: 'スピナー', sub: 'スプラッシュシールド', special: 'マルチミサイル' },
  { id: 'hydrant', name: 'ハイドラント', category: 'スピナー', sub: 'スプリンクラー', special: 'グレートバリア' },
  { id: 'kugelschreiber', name: 'クーゲルシュライバー', category: 'スピナー', sub: 'スプラッシュボム', special: 'カニタンク' },
  { id: 'examiner', name: 'イグザミナー', category: 'スピナー', sub: 'トーピード', special: 'ホタルパニック' },

  // フデ
  { id: 'pablo', name: 'パブロ', category: 'フデ', sub: 'スプリンクラー', special: 'ウルトラチャクチ' },
  { id: 'hokusai', name: 'ホクサイ', category: 'フデ', sub: 'スプラッシュボム', special: 'ウルトラチャクチ' },

  // ワイパー
  { id: 'jimwiper', name: 'ジムワイパー', category: 'ワイパー', sub: 'クイックボム', special: 'グレートバリア' },
  { id: 'dentalwiper_mint', name: 'デンタルワイパーミント', category: 'ワイパー', sub: 'スプラッシュシールド', special: 'ショクワンダー' },
  { id: 'drivewiper', name: 'ドライブワイパー', category: 'ワイパー', sub: 'スプラッシュボム', special: 'メガホンレーザー5.1ch' },
  { id: 'orderwiper_replica', name: 'オーダーワイパー レプリカ', category: 'ワイパー', sub: 'ポイズンミスト', special: 'デコイチラシ' },

  // シェルター
  { id: 'parashelter', name: 'パラシェルター', category: 'シェルター', sub: 'スプラッシュボム', special: 'ウルトラハンコ' },
  { id: 'campingshelter', name: 'キャンピングシェルター', category: 'シェルター', sub: 'スプリンクラー', special: 'カニタンク' },
  { id: 'spygadget', name: 'スパイガジェット', category: 'シェルター', sub: 'スプラッシュボム', special: 'グレートバリア' },

  // ストリンガー
  { id: 'tristringer', name: 'トライストリンガー', category: 'ストリンガー', sub: 'クイックボム', special: 'ホップソナー' },
  { id: 'lact450', name: 'LACT-450', category: 'ストリンガー', sub: 'ロボットボム', special: 'カニタンク' },

  // マニューバー
  { id: 'splat_dualies', name: 'スプラマニューバー', category: 'マニューバー', sub: 'ロボットボム', special: 'カニタンク' },
  { id: 'spattery', name: 'スパッタリー', category: 'マニューバー', sub: 'スプリンクラー', special: 'スーパーチャクチ' },
  { id: 'dual_sweeper', name: 'デュアルスイーパー', category: 'マニューバー', sub: 'スプラッシュシールド', special: 'ジェットパック' },
  { id: 'kelvin525', name: 'ケルビン525', category: 'マニューバー', sub: 'キューバンボム', special: 'アメフラシ' },
  { id: 'quad_hopper_black', name: 'クアッドホッパーブラック', category: 'マニューバー', sub: 'トーピード', special: 'マルチミサイル' },
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

/** 直近に保存したブキ名を取得 (自動記録/手動入力の未入力時のデフォルト用) */
export function getLastWeaponName(): string | null {
  const recent = getRecentWeapons(1);
  return recent.length > 0 ? recent[0].name : null;
}
