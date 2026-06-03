import cv2
import os
import numpy as np
import argparse
import sys
from datetime import datetime

# ==========================================
# 【パッチ】OpenCVの日本語パス対応
# ==========================================
def imread_jp(filename, flags=cv2.IMREAD_COLOR):
    """日本語パス対応の画像読み込み"""
    try:
        n = np.fromfile(filename, np.uint8)
        img = cv2.imdecode(n, flags)
        return img
    except Exception as e:
        return None

def imwrite_jp(filename, img, params=None):
    """日本語パス対応の画像書き出し"""
    try:
        ext = os.path.splitext(filename)[1]
        result, n = cv2.imencode(ext, img, params)
        if result:
            with open(filename, mode='w+b') as f:
                n.tofile(f)
            return True
        else:
            return False
    except Exception as e:
        return False
# ==========================================

# ==========================================
# 【高速化】Numpy配列スライシングによる空間一括分割
# ==========================================
def calculate_spatial_histogram(image, bins=32, grid_x=4, grid_y=4):
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    h, w = gray.shape
    
    # 画像を縦横に均等分割できるサイズにリサイズ（端数切り捨て）
    new_h = h - (h % grid_y)
    new_w = w - (w % grid_x)
    gray = cv2.resize(gray, (new_w, new_h))
    
    # Numpyのスライシングを使って一瞬で16個のブロックに分割
    blocks = [gray[y:y+new_h//grid_y, x:x+new_w//grid_x] 
              for y in range(0, new_h, new_h//grid_y) 
              for x in range(0, new_w, new_w//grid_x)]
    
    hist_features = []
    # 各ブロックのヒストグラムを計算
    for block in blocks:
        hist = cv2.calcHist([block], [0], None, [bins], [0, 256])
        hist_features.extend(hist.flatten())
        
    hist_features = np.array(hist_features, dtype=np.float32).reshape(-1, 1)
    cv2.normalize(hist_features, hist_features, 0, 1, cv2.NORM_MINMAX)
    return hist_features

def process_local_videos(video_paths, ref_paths, threshold, interval, cooldown, output_dir, debug_mode):
    os.makedirs(output_dir, exist_ok=True)

    refs = []
    if ref_paths:
        print(f"【モード】類似度抽出 (空間分割) - 閾値: {threshold}")
        if debug_mode: print("【⚠ デバッグモード有効】詳細なスコアログを出力します")
        for path in ref_paths:
            img = imread_jp(path)
            if img is None:
                print(f"エラー: お手本画像 '{path}' が読み込めません。", file=sys.stderr)
                continue
            hist = calculate_spatial_histogram(img)
            base_name = os.path.splitext(os.path.basename(path))[0]
            refs.append({
                'path': path, 'name': base_name, 'hist': hist,
                'saved_count': 0, 'last_time': -float('inf')
            })
            print(f"  -> 登録完了: {base_name}")
            
        if not refs:
            print("有効なお手本がありません。", file=sys.stderr)
            sys.exit(1)
    else:
        print("【モード】定期スクショ: オン")

    total_saved_periodic = 0

    for index, video_path in enumerate(video_paths):
        video_filename = os.path.basename(video_path)
        video_id = os.path.splitext(video_filename)[0]
        
        print(f"\n{'='*50}\n[{index+1}/{len(video_paths)}] 処理開始: {video_filename}\n(パス: {video_path})\n{'='*50}")

        try:
            cap = cv2.VideoCapture(video_path)
            
            if not cap.isOpened():
                print(f"⚠️ 警告: 動画ファイルが開けませんでした -> {video_filename}", file=sys.stderr)
                continue

            fps = cap.get(cv2.CAP_PROP_FPS)
            if fps <= 0 or np.isnan(fps): fps = 30.0

            frame_interval = int(round(fps * interval))
            if frame_interval < 1: frame_interval = 1

            frame_count = 0
            for ref in refs:
                ref['vid_saved_count'] = 0
                ref['last_time'] = -float('inf')

            while True:
                # 【高速化】必要なフレームの時だけ read() (デコードして画像を取得)
                if frame_count % frame_interval == 0:
                    ret, frame = cap.read()
                    if not ret: break

                    current_sec = frame_count / fps
                    sec_int = int(current_sec)
                    m, s = divmod(sec_int, 60)
                    h, m = divmod(m, 60)
                    ms = int((current_sec - sec_int) * 100)
                    
                    time_str = f"{h:02d}h{m:02d}m{s:02d}s_{ms:02d}ms"
                    vid_prefix = f"vid{index+1:04d}"
                    real_time = datetime.now().strftime('%H%M%S%f')[:-3]

                    if refs:
                        target_hist = calculate_spatial_histogram(frame)
                        best_match = None
                        max_sim = -1.0
                        debug_scores = []

                        for ref in refs:
                            sim = cv2.compareHist(ref['hist'], target_hist, cv2.HISTCMP_CORREL)
                            debug_scores.append(f"{ref['name']}:{sim:.2f}")
                            if sim > max_sim:
                                max_sim = sim
                                best_match = ref

                        if debug_mode:
                            log_line = f"[{time_str}] " + " | ".join(debug_scores) + f"  => 暫定勝者: {best_match['name']}({max_sim:.2f})"
                            
                            if max_sim >= threshold:
                                time_since_last = current_sec - best_match['last_time']
                                if time_since_last >= cooldown:
                                    fn = os.path.join(output_dir, f"{vid_prefix}_{video_id}_{time_str}_match_{best_match['name']}_sim{max_sim:.3f}_{real_time}.jpg")
                                    imwrite_jp(fn, frame)
                                    best_match['vid_saved_count'] += 1
                                    best_match['saved_count'] += 1
                                    best_match['last_time'] = current_sec
                                    print(log_line + " 🟢 保存成功！")
                                else:
                                    print(log_line + f" 🟡 スキップ (クールダウン中: 残り{cooldown - time_since_last:.1f}秒)")
                            else:
                                print(log_line + f" 🔴 スキップ (閾値 {threshold} 未満)")

                        else:
                            if max_sim >= threshold:
                                if current_sec - best_match['last_time'] >= cooldown:
                                    fn = os.path.join(output_dir, f"{vid_prefix}_{video_id}_{time_str}_match_{best_match['name']}_sim{max_sim:.3f}_{real_time}.jpg")
                                    imwrite_jp(fn, frame)
                                    best_match['vid_saved_count'] += 1
                                    best_match['saved_count'] += 1
                                    best_match['last_time'] = current_sec
                            
                            counts_str = " ".join([f"[{r['name']}:{r['vid_saved_count']}]" for r in refs])
                            print(f"\r[{time_str}] 探索中 (勝者 {best_match['name']}:{max_sim:.2f}) | {counts_str} ", end="")

                    else:
                        fn = os.path.join(output_dir, f"{vid_prefix}_{video_id}_{time_str}_periodic_{real_time}.jpg")
                        imwrite_jp(fn, frame)
                        total_saved_periodic += 1
                        if debug_mode:
                            print(f"[{time_str}] 🟢 定期保存成功 (計:{total_saved_periodic})")
                        else:
                            print(f"\r[{time_str}] 定期保存 | 計:{total_saved_periodic}", end="")

                # 【高速化】不要なフレームの時は grab() (デコードせずにポインタだけ爆速で進める)
                else:
                    ret = cap.grab()
                    if not ret: break

                frame_count += 1
                
            cap.release()
            if refs:
                counts_str = ", ".join([f"{r['name']}: {r['vid_saved_count']}枚" for r in refs])
                print(f"\n-> 完了！ ({counts_str})")
            else:
                print(f"\n-> 完了！ (定期保存: {total_saved_periodic}枚)")

        except KeyboardInterrupt:
            ans = input("\n次の動画に進みますか？ (y/n): ")
            if ans.lower() != 'y': break
        except Exception as e:
            print(f"\nエラー発生: {e}", file=sys.stderr)

    print(f"\n{'='*50}")
    if refs:
        print("全プロセス完了！ 抽出結果:")
        for r in refs: print(f" - {r['name']}: {r['saved_count']}枚")
    else:
        print(f"全プロセス完了！ 定期保存: {total_saved_periodic}枚")
    print(f"保存先 -> '{output_dir}'")

if __name__ == "__main__":
    script_dir = os.path.dirname(os.path.abspath(__file__))
    ts = datetime.now().strftime("%Y%m%d_%H%M%S")
    default_out = os.path.join(script_dir, f"extract_frames_{ts}")

    parser = argparse.ArgumentParser(description="超高速・日本語パス完全対応 動画画像スクレイピングツール")
    parser.add_argument("-d", "--dir", type=str, required=True, help="対象の動画が格納されているディレクトリのパス")
    parser.add_argument("-r", "--refs", type=str, nargs='*', default=[], help="お手本画像のパス (何枚でも)")
    parser.add_argument("-t", "--threshold", type=float, default=0.65, help="閾値 (デフォルト: 0.65)")
    parser.add_argument("-i", "--interval", type=float, default=1.0, help="スキャン間隔 (デフォルト: 1.0)")
    parser.add_argument("-c", "--cooldown", type=float, default=30.0, help="クールダウン (デフォルト: 30.0)")
    parser.add_argument("-o", "--output", type=str, default=default_out, help="保存先")
    parser.add_argument("-db", "--debug", action="store_true", help="各フレームの全スコアと判定理由を出力します")

    args = parser.parse_args()
    
    target_dir = args.dir
    if not os.path.isdir(target_dir):
        print(f"エラー: 指定されたディレクトリが見つかりません: {target_dir}", file=sys.stderr)
        sys.exit(1)

    valid_extensions = ('.mp4', '.mkv', '.avi', '.mov', '.webm', '.flv', '.wmv')
    
    video_files = []
    for root, dirs, files in os.walk(target_dir):
        for f in files:
            if f.lower().endswith(valid_extensions):
                video_files.append(os.path.join(root, f))
                
    video_files.sort()

    if not video_files:
        print(f"エラー: 指定されたディレクトリ内に動画ファイルが見つかりません。({target_dir})", file=sys.stderr)
        sys.exit(1)

    print(f"ディレクトリ読み込み完了: 計 {len(video_files)} 本の動画を処理します。")

    process_local_videos(video_files, args.refs, args.threshold, args.interval, args.cooldown, args.output, args.debug)