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
  // ── シューター ──────────────────────────────────────────────────
  { id: 'wakaba',                  name: 'わかばシューター',         category: 'シューター', sub: 'スプラッシュボム',    special: 'グレートバリア' },
  { id: 'momiji',                  name: 'もみじシューター',         category: 'シューター', sub: 'トラップ',            special: 'ナイスダマ' },
  { id: 'bold_marker',             name: 'ボールドマーカー',         category: 'シューター', sub: 'スプラッシュボム',    special: 'トリプルトルネード' },
  { id: 'bold_marker_neo',         name: 'ボールドマーカーネオ',     category: 'シューター', sub: 'キューバンボム',       special: 'カニタンク' },
  { id: 'sharp_marker',            name: 'シャープマーカー',         category: 'シューター', sub: 'キューバンボム',       special: 'ウルトラハンコ' },
  { id: 'sharp_marker_neo',        name: 'シャープマーカーネオ',     category: 'シューター', sub: 'スプリンクラー',      special: 'キューインキ' },
  { id: 'sharp_marker_geck',       name: 'シャープマーカーGECK',    category: 'シューター', sub: 'ポイズンミスト',      special: 'ウルトラショット' },
  { id: 'splat_shooter',           name: 'スプラシューター',         category: 'シューター', sub: 'スプラッシュボム',    special: 'ウルトラショット' },
  { id: 'splat_shooter_collabo',   name: 'スプラシューターコラボ',   category: 'シューター', sub: 'キューバンボム',       special: 'アメフラシ' },
  { id: 'splat_shooter_kou',       name: 'スプラシューター煌',       category: 'シューター', sub: 'ポイントセンサー',    special: 'マルチミサイル' },
  { id: 'n_zap_85',                name: "N-ZAP'85",                 category: 'シューター', sub: 'スプラッシュボム',    special: 'ショクワンダー' },
  { id: 'n_zap_89',                name: "N-ZAP'89",                 category: 'シューター', sub: 'ポイントセンサー',    special: 'グレートバリア' },
  { id: 'promodeler_mg',           name: 'プロモデラーMG',           category: 'シューター', sub: 'ポイントセンサー',    special: 'キューインキ' },
  { id: 'promodeler_rg',           name: 'プロモデラーRG',           category: 'シューター', sub: 'スプリンクラー',      special: 'キューインキ' },
  { id: 'promodeler_sai',          name: 'プロモデラー彩',           category: 'シューター', sub: 'カーリングボム',      special: 'ジェットパック' },
  { id: 'jet_sweeper',             name: 'ジェットスイーパー',       category: 'シューター', sub: 'ポイントセンサー',    special: 'ジェットパック' },
  { id: 'jet_sweeper_custom',      name: 'ジェットスイーパーカスタム', category: 'シューター', sub: 'ロボットボム',      special: 'スーパーチャクチ' },
  { id: 'gallon_52',               name: '.52ガロン',                category: 'シューター', sub: 'カーリングボム',      special: 'ウルトラチャクチ' },
  { id: 'gallon_52_deco',          name: '.52ガロンデコ',            category: 'シューター', sub: 'スプラッシュシールド', special: 'ショクワンダー' },
  { id: 'gallon_96',               name: '.96ガロン',                category: 'シューター', sub: 'タンサンボム',        special: 'メガホンレーザー5.1ch' },
  { id: 'gallon_96_deco',          name: '.96ガロンデコ',            category: 'シューター', sub: 'スプリンクラー',      special: 'ジェットパック' },
  { id: 'gallon_96_tsume',         name: '.96ガロン爪',              category: 'シューター', sub: 'タンサンボム',        special: 'サメライド' },
  { id: 'l3_reelgun',              name: 'L3リールガン',             category: 'シューター', sub: 'キューバンボム',       special: 'ウルトラチャクチ' },
  { id: 'l3_reelgun_d',            name: 'L3リールガンD',            category: 'シューター', sub: 'カーリングボム',      special: 'ホップソナー' },
  { id: 'l3_reelgun_haku',         name: 'L3リールガン箔',           category: 'シューター', sub: 'スプラッシュボム',    special: 'トリプルトルネード' },
  { id: 'h3_reelgun',              name: 'H3リールガン',             category: 'シューター', sub: 'ポイントセンサー',    special: 'メガホンレーザー5.1ch' },
  { id: 'h3_reelgun_d',            name: 'H3リールガンD',            category: 'シューター', sub: 'スプラッシュシールド', special: 'ウルトラチャクチ' },
  { id: 'h3_reelgun_snak',         name: 'H3リールガンSNAK',        category: 'シューター', sub: 'ポイントセンサー',    special: 'ナイスダマ' },
  { id: 'prime_shooter',           name: 'プライムシューター',       category: 'シューター', sub: 'トーピード',          special: 'ウルトラショット' },
  { id: 'prime_shooter_collabo',   name: 'プライムシューターコラボ', category: 'シューター', sub: 'スプリンクラー',      special: 'ショクワンダー' },
  { id: 'bottle_geyser',           name: 'ボトルガイザー',           category: 'シューター', sub: 'スプラッシュボム',    special: 'グレートバリア' },
  { id: 'bottle_geyser_foil',      name: 'ボトルガイザーフォイル',   category: 'シューター', sub: 'ジャンプビーコン',    special: 'グレートバリア' },
  { id: 'space_shooter',           name: 'スペースシューター',       category: 'シューター', sub: 'ロボットボム',        special: 'ナイスダマ' },
  { id: 'hero_shooter_replica',    name: 'ヒーローシューターレプリカ', category: 'シューター', sub: 'スプラッシュボム',  special: 'トリプルトルネード' },
  { id: 'octo_shooter_replica',    name: 'オクタシューターレプリカ', category: 'シューター', sub: 'ポイズンミスト',      special: 'カニタンク' },

  // ── ローラー ────────────────────────────────────────────────────
  { id: 'splat_roller',            name: 'スプラローラー',           category: 'ローラー', sub: 'カーリングボム',        special: 'ナイスダマ' },
  { id: 'splat_roller_collabo',    name: 'スプラローラーコラボ',     category: 'ローラー', sub: 'キューバンボム',         special: 'グレートバリア' },
  { id: 'carbon_roller',           name: 'カーボンローラー',         category: 'ローラー', sub: 'スプリンクラー',        special: 'ジェットパック' },
  { id: 'carbon_roller_deco',      name: 'カーボンローラーデコ',     category: 'ローラー', sub: 'トーピード',            special: 'ウルトラハンコ' },
  { id: 'carbon_roller_angl',      name: 'カーボンローラーANGL',    category: 'ローラー', sub: 'スプラッシュボム',       special: 'ショクワンダー' },
  { id: 'dynamo_roller',           name: 'ダイナモローラー',         category: 'ローラー', sub: 'トラップ',              special: 'メガホンレーザー5.1ch' },
  { id: 'dynamo_roller_tesla',     name: 'ダイナモローラーテスラ',   category: 'ローラー', sub: 'スプラッシュボム',       special: 'カニタンク' },
  { id: 'dynamo_roller_mei',       name: 'ダイナモローラー冥',       category: 'ローラー', sub: 'ポイントセンサー',       special: 'ホップソナー' },
  { id: 'variable_roller',         name: 'ヴァリアブルローラー',     category: 'ローラー', sub: 'ポイントセンサー',       special: 'ホップソナー' },
  { id: 'variable_roller_foil',    name: 'ヴァリアブルローラーフォイル', category: 'ローラー', sub: 'スプラッシュシールド', special: 'カニタンク' },
  { id: 'wide_roller',             name: 'ワイドローラー',           category: 'ローラー', sub: 'ポイズンミスト',        special: 'ウルトラチャクチ' },
  { id: 'wide_roller_collabo',     name: 'ワイドローラーコラボ',     category: 'ローラー', sub: 'ジャンプビーコン',       special: 'デコイチラシ' },
  { id: 'wide_roller_waku',        name: 'ワイドローラー惑',         category: 'ローラー', sub: 'タンサンボム',           special: 'グレートバリア' },

  // ── ブラスター ──────────────────────────────────────────────────
  { id: 'nova_blaster',            name: 'ノヴァブラスター',         category: 'ブラスター', sub: 'ポイントセンサー',    special: 'アメフラシ' },
  { id: 'nova_blaster_neo',        name: 'ノヴァブラスターネオ',     category: 'ブラスター', sub: 'タンサンボム',        special: 'デコイチラシ' },
  { id: 'clash_blaster',           name: 'クラッシュブラスター',     category: 'ブラスター', sub: 'スプラッシュボム',    special: 'カニタンク' },
  { id: 'clash_blaster_neo',       name: 'クラッシュブラスターネオ', category: 'ブラスター', sub: 'ロボットボム',        special: 'デコイチラシ' },
  { id: 'hot_blaster',             name: 'ホットブラスター',         category: 'ブラスター', sub: 'スプリンクラー',      special: 'アメフラシ' },
  { id: 'hot_blaster_en',          name: 'ホットブラスター艶',       category: 'ブラスター', sub: 'タンサンボム',        special: 'ホップソナー' },
  { id: 'long_blaster',            name: 'ロングブラスター',         category: 'ブラスター', sub: 'スプラッシュシールド', special: 'グレートバリア' },
  { id: 'long_blaster_custom',     name: 'ロングブラスターカスタム', category: 'ブラスター', sub: 'ロボットボム',        special: 'マルチミサイル' },
  { id: 's_blast_92',              name: 'S-BLAST92',               category: 'ブラスター', sub: 'スプラッシュシールド', special: 'キューインキ' },
  { id: 'rapid_blaster',           name: 'ラピッドブラスター',       category: 'ブラスター', sub: 'トーピード',          special: 'ナイスダマ' },
  { id: 'rapid_blaster_deco',      name: 'ラピッドブラスターデコ',   category: 'ブラスター', sub: 'ポイズンミスト',      special: 'マルチミサイル' },
  { id: 'quick_blaster',           name: 'クイックブラスター',       category: 'ブラスター', sub: 'ロボットボム',        special: 'カニタンク' },
  { id: 'r_blaster_elite',         name: 'Rブラスターエリート',      category: 'ブラスター', sub: 'ポイントセンサー',    special: 'ジェットパック' },
  { id: 'r_blaster_elite_deco',    name: 'Rブラスターエリートデコ',  category: 'ブラスター', sub: 'トラップ',            special: 'サメライド' },

  // ── チャージャー ────────────────────────────────────────────────
  { id: 'splat_charger',           name: 'スプラチャージャー',       category: 'チャージャー', sub: 'スプラッシュボム',  special: 'マルチミサイル' },
  { id: 'splat_charger_collabo',   name: 'スプラチャージャーコラボ', category: 'チャージャー', sub: 'スプリンクラー',    special: 'マルチミサイル' },
  { id: 'splat_scope',             name: 'スプラスコープ',           category: 'チャージャー', sub: 'スプラッシュボム',  special: 'マルチミサイル' },
  { id: 'splat_scope_collabo',     name: 'スプラスコープコラボ',     category: 'チャージャー', sub: 'スプリンクラー',    special: 'マルチミサイル' },
  { id: 'squiclean_a',             name: 'スクイックリンα',          category: 'チャージャー', sub: 'ポイントセンサー',  special: 'スーパーチャクチ' },
  { id: 'squiclean_b',             name: 'スクイックリンβ',          category: 'チャージャー', sub: 'ロボットボム',      special: 'グレートバリア' },
  { id: 'liter_4k',                name: 'リッター4K',               category: 'チャージャー', sub: 'ポイントセンサー',  special: 'ウルトラチャクチ' },
  { id: 'liter_4k_custom',         name: 'リッター4Kカスタム',       category: 'チャージャー', sub: 'スプラッシュシールド', special: 'マルチミサイル' },
  { id: 'liter_4k_scope',          name: '4Kスコープ',               category: 'チャージャー', sub: 'ポイントセンサー',  special: 'ウルトラチャクチ' },
  { id: 'liter_4k_scope_custom',   name: '4Kスコープカスタム',       category: 'チャージャー', sub: 'スプラッシュシールド', special: 'マルチミサイル' },
  { id: 'bamboozler_mk1',          name: '14式竹筒銃・甲',           category: 'チャージャー', sub: 'カーリングボム',    special: 'ウルトラチャクチ' },
  { id: 'bamboozler_mk2',          name: '14式竹筒銃・乙',           category: 'チャージャー', sub: 'スプリンクラー',    special: 'ウルトラハンコ' },
  { id: 'goo_tuber',               name: 'ソイチューバー',           category: 'チャージャー', sub: 'ポイントセンサー',  special: 'ジェットパック' },
  { id: 'goo_tuber_custom',        name: 'ソイチューバーカスタム',   category: 'チャージャー', sub: 'タンサンボム',      special: 'デコイチラシ' },
  { id: 'order_charger_replica',   name: 'オーダーチャージャーレプリカ', category: 'チャージャー', sub: 'スプラッシュボム', special: 'マルチミサイル' },

  // ── スロッシャー ────────────────────────────────────────────────
  { id: 'bucket_slosher',          name: 'バケットスロッシャー',     category: 'スロッシャー', sub: 'スプラッシュボム',  special: 'ホップソナー' },
  { id: 'bucket_slosher_deco',     name: 'バケットスロッシャーデコ', category: 'スロッシャー', sub: 'ロボットボム',      special: 'グレートバリア' },
  { id: 'hissen',                  name: 'ヒッセン',                 category: 'スロッシャー', sub: 'スプラッシュボム',  special: 'ウルトラチャクチ' },
  { id: 'hissen_hue',              name: 'ヒッセン・ヒュー',         category: 'スロッシャー', sub: 'トーピード',        special: 'アメフラシ' },
  { id: 'hissen_ash',              name: 'ヒッセンASH',              category: 'スロッシャー', sub: 'スプリンクラー',    special: 'カニタンク' },
  { id: 'screw_slosher',           name: 'スクリュースロッシャー',   category: 'スロッシャー', sub: 'スプリンクラー',    special: 'カニタンク' },
  { id: 'screw_slosher_neo',       name: 'スクリュースロッシャーネオ', category: 'スロッシャー', sub: 'タンサンボム',    special: 'マルチミサイル' },
  { id: 'over_flosher',            name: 'オーバーフロッシャー',     category: 'スロッシャー', sub: 'タンサンボム',      special: 'デコイチラシ' },
  { id: 'over_flosher_deco',       name: 'オーバーフロッシャーデコ', category: 'スロッシャー', sub: 'ポイントセンサー',  special: 'マルチミサイル' },
  { id: 'explosher',               name: 'エクスプロッシャー',       category: 'スロッシャー', sub: 'ポイントセンサー',  special: 'カニタンク' },
  { id: 'explosher_custom',        name: 'エクスプロッシャーカスタム', category: 'スロッシャー', sub: 'スプラッシュシールド', special: 'ジェットパック' },
  { id: 'mopurin',                 name: 'モップリン',               category: 'スロッシャー', sub: 'ジャンプビーコン',  special: 'ショクワンダー' },
  { id: 'mopurin_d',               name: 'モップリンD',              category: 'スロッシャー', sub: 'スプリンクラー',    special: 'ジェットパック' },
  { id: 'mopurin_kado',            name: 'モップリン角',             category: 'スロッシャー', sub: 'トーピード',        special: 'ナイスダマ' },
  { id: 'order_slosher_replica',   name: 'オーダースロッシャーレプリカ', category: 'スロッシャー', sub: 'スプラッシュボム', special: 'ホップソナー' },

  // ── スピナー ────────────────────────────────────────────────────
  { id: 'splat_spinner',           name: 'スプラスピナー',           category: 'スピナー', sub: 'スプラッシュボム',      special: 'ジェットパック' },
  { id: 'splat_spinner_collabo',   name: 'スプラスピナーコラボ',     category: 'スピナー', sub: 'ポイントセンサー',      special: 'サメライド' },
  { id: 'splat_spinner_pytn',      name: 'スプラスピナーPYTN',      category: 'スピナー', sub: 'スプラッシュシールド',  special: 'ジェットパック' },
  { id: 'barrel_spinner',          name: 'バレルスピナー',           category: 'スピナー', sub: 'スプラッシュシールド',  special: 'マルチミサイル' },
  { id: 'barrel_spinner_deco',     name: 'バレルスピナーデコ',       category: 'スピナー', sub: 'スプリンクラー',        special: 'ジェットパック' },
  { id: 'hydrant',                 name: 'ハイドラント',             category: 'スピナー', sub: 'スプリンクラー',        special: 'グレートバリア' },
  { id: 'hydrant_custom',          name: 'ハイドラントカスタム',     category: 'スピナー', sub: 'ポイズンミスト',        special: 'マルチミサイル' },
  { id: 'hydrant_atu',             name: 'ハイドラント圧',           category: 'スピナー', sub: 'ロボットボム',          special: 'ウルトラショット' },
  { id: 'kugelschreiber',          name: 'クーゲルシュライバー',     category: 'スピナー', sub: 'スプラッシュボム',      special: 'カニタンク' },
  { id: 'kugelschreiber_hue',      name: 'クーゲルシュライバー・ヒュー', category: 'スピナー', sub: 'スプラッシュシールド', special: 'ショクワンダー' },
  { id: 'nautilus_47',             name: 'ノーチラス47',             category: 'スピナー', sub: 'スプラッシュシールド',  special: 'トリプルトルネード' },
  { id: 'nautilus_79',             name: 'ノーチラス79',             category: 'スピナー', sub: 'ポイントセンサー',      special: 'マルチミサイル' },
  { id: 'examiner',                name: 'イグザミナー',             category: 'スピナー', sub: 'トーピード',            special: 'ホタルパニック' },
  { id: 'examiner_hue',            name: 'イグザミナー・ヒュー',     category: 'スピナー', sub: 'スプリンクラー',        special: 'ウルトラチャクチ' },
  { id: 'order_spinner_replica',   name: 'オーダースピナーレプリカ', category: 'スピナー', sub: 'スプラッシュボム',      special: 'ジェットパック' },

  // ── フデ ────────────────────────────────────────────────────────
  { id: 'pablo',                   name: 'パブロ',                   category: 'フデ', sub: 'スプリンクラー',           special: 'ウルトラチャクチ' },
  { id: 'pablo_hue',               name: 'パブロ・ヒュー',           category: 'フデ', sub: 'ポイントセンサー',         special: 'ウルトラチャクチ' },
  { id: 'hokusai',                 name: 'ホクサイ',                 category: 'フデ', sub: 'スプラッシュボム',         special: 'ウルトラチャクチ' },
  { id: 'hokusai_hue',             name: 'ホクサイ・ヒュー',         category: 'フデ', sub: 'ポイントセンサー',         special: 'サメライド' },
  { id: 'hokusai_sui',             name: 'ホクサイ彗',               category: 'フデ', sub: 'スプリンクラー',           special: 'ナイスダマ' },

  // ── ワイパー ────────────────────────────────────────────────────
  { id: 'jimwiper',                name: 'ジムワイパー',             category: 'ワイパー', sub: 'クイックボム',           special: 'グレートバリア' },
  { id: 'jimwiper_hue',            name: 'ジムワイパー・ヒュー',     category: 'ワイパー', sub: 'ポイントセンサー',       special: 'ショクワンダー' },
  { id: 'jimwiper_fuu',            name: 'ジムワイパー封',           category: 'ワイパー', sub: 'ロボットボム',           special: 'マルチミサイル' },
  { id: 'dentalwiper_mint',        name: 'デンタルワイパーミント',   category: 'ワイパー', sub: 'スプラッシュシールド',   special: 'ショクワンダー' },
  { id: 'dentalwiper_sumi',        name: 'デンタルワイパースミ',     category: 'ワイパー', sub: 'スプラッシュシールド',   special: 'デコイチラシ' },
  { id: 'drivewiper',              name: 'ドライブワイパー',         category: 'ワイパー', sub: 'スプラッシュボム',       special: 'メガホンレーザー5.1ch' },
  { id: 'drivewiper_deco',         name: 'ドライブワイパーデコ',     category: 'ワイパー', sub: 'トーピード',             special: 'アメフラシ' },
  { id: 'orderwiper_replica',      name: 'オーダーワイパー レプリカ', category: 'ワイパー', sub: 'ポイズンミスト',        special: 'デコイチラシ' },

  // ── シェルター ──────────────────────────────────────────────────
  { id: 'parashelter',             name: 'パラシェルター',           category: 'シェルター', sub: 'スプラッシュボム',    special: 'ウルトラハンコ' },
  { id: 'parashelter_sorella',     name: 'パラシェルターソレーラ',   category: 'シェルター', sub: 'ロボットボム',        special: 'ジェットパック' },
  { id: 'tenta_brella_a',          name: '24式張替傘・甲',           category: 'シェルター', sub: 'ジャンプビーコン',    special: 'ショクワンダー' },
  { id: 'tenta_brella_b',          name: '24式張替傘・乙',           category: 'シェルター', sub: 'スプリンクラー',      special: 'ウルトラハンコ' },
  { id: 'campingshelter',          name: 'キャンピングシェルター',   category: 'シェルター', sub: 'スプリンクラー',      special: 'カニタンク' },
  { id: 'campingshelter_sorella',  name: 'キャンピングシェルターソレーラ', category: 'シェルター', sub: 'トーピード',    special: 'マルチミサイル' },
  { id: 'campingshelter_crem',     name: 'キャンピングシェルターCREM', category: 'シェルター', sub: 'スプラッシュシールド', special: 'ウルトラショット' },
  { id: 'spygadget',               name: 'スパイガジェット',         category: 'シェルター', sub: 'スプラッシュボム',    special: 'グレートバリア' },
  { id: 'spygadget_sorella',       name: 'スパイガジェットソレーラ', category: 'シェルター', sub: 'スプリンクラー',      special: 'グレートバリア' },
  { id: 'spygadget_ryou',          name: 'スパイガジェット繚',       category: 'シェルター', sub: 'ポイントセンサー',    special: 'ナイスダマ' },

  // ── ストリンガー ────────────────────────────────────────────────
  { id: 'tristringer',             name: 'トライストリンガー',       category: 'ストリンガー', sub: 'クイックボム',      special: 'ホップソナー' },
  { id: 'tristringer_collabo',     name: 'トライストリンガーコラボ', category: 'ストリンガー', sub: 'ロボットボム',      special: 'マルチミサイル' },
  { id: 'tristringer_tou',         name: 'トライストリンガー燈',     category: 'ストリンガー', sub: 'タンサンボム',      special: 'ジェットパック' },
  { id: 'lact450',                 name: 'LACT-450',                 category: 'ストリンガー', sub: 'ロボットボム',      special: 'カニタンク' },
  { id: 'lact450_deco',            name: 'LACT-450デコ',             category: 'ストリンガー', sub: 'スプリンクラー',    special: 'ホップソナー' },
  { id: 'wellstring_v',            name: 'フルイドV',                category: 'ストリンガー', sub: 'スプラッシュシールド', special: 'ウルトラショット' },
  { id: 'wellstring_v_custom',     name: 'フルイドVカスタム',        category: 'ストリンガー', sub: 'ポイントセンサー',  special: 'ホップソナー' },
  { id: 'order_stringer_replica',  name: 'オーダーストリンガーレプリカ', category: 'ストリンガー', sub: 'クイックボム',  special: 'ホップソナー' },

  // ── マニューバー ────────────────────────────────────────────────
  { id: 'splat_dualies',           name: 'スプラマニューバー',       category: 'マニューバー', sub: 'ロボットボム',      special: 'カニタンク' },
  { id: 'splat_dualies_collabo',   name: 'スプラマニューバーコラボ', category: 'マニューバー', sub: 'スプラッシュボム',  special: 'ジェットパック' },
  { id: 'splat_dualies_you',       name: 'スプラマニューバー耀',     category: 'マニューバー', sub: 'ポイントセンサー',  special: 'マルチミサイル' },
  { id: 'spattery',                name: 'スパッタリー',             category: 'マニューバー', sub: 'スプリンクラー',    special: 'スーパーチャクチ' },
  { id: 'spattery_hue',            name: 'スパッタリー・ヒュー',     category: 'マニューバー', sub: 'ポイントセンサー',  special: 'マルチミサイル' },
  { id: 'dual_sweeper',            name: 'デュアルスイーパー',       category: 'マニューバー', sub: 'スプラッシュシールド', special: 'ジェットパック' },
  { id: 'dual_sweeper_custom',     name: 'デュアルスイーパーカスタム', category: 'マニューバー', sub: 'スプラッシュボム', special: 'マルチミサイル' },
  { id: 'dual_sweeper_tei',        name: 'デュアルスイーパー蹄',     category: 'マニューバー', sub: 'タンサンボム',      special: 'ショクワンダー' },
  { id: 'kelvin525',               name: 'ケルビン525',              category: 'マニューバー', sub: 'キューバンボム',    special: 'アメフラシ' },
  { id: 'kelvin525_deco',          name: 'ケルビン525デコ',          category: 'マニューバー', sub: 'ポイントセンサー',  special: 'マルチミサイル' },
  { id: 'quad_hopper_black',       name: 'クアッドホッパーブラック', category: 'マニューバー', sub: 'トーピード',        special: 'マルチミサイル' },
  { id: 'quad_hopper_white',       name: 'クアッドホッパーホワイト', category: 'マニューバー', sub: 'スプラッシュシールド', special: 'ウルトラチャクチ' },
  { id: 'gaen_ff',                 name: 'ガエンFF',                 category: 'マニューバー', sub: 'トーピード',        special: 'ウルトラショット' },
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
