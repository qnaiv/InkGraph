import cv2
import numpy as np
import onnxruntime as ort
import os
import glob
import argparse

# ==========================================
# 固定設定エリア
# ==========================================
INPUT_WIDTH = 640
INPUT_HEIGHT = 640
CONF_THRESHOLD = 0.5

# InkGraphのONNXモデル（10クラス）の7番目（MyArrow）を狙う
TARGET_CLASS_INDEX = 6  
# ==========================================

class ArrowDetector:
    def __init__(self, model_path):
        providers = ['CUDAExecutionProvider', 'CPUExecutionProvider'] if ort.get_device() == 'GPU' else ['CPUExecutionProvider']
        self.session = ort.InferenceSession(model_path, providers=providers)
        self.input_name = self.session.get_inputs()[0].name

    def preprocess(self, img_bgr):
        h, w = img_bgr.shape[:2]
        scale = min(INPUT_WIDTH / w, INPUT_HEIGHT / h)
        nw, nh = int(w * scale), int(h * scale)
        img_resized = cv2.resize(img_bgr, (nw, nh))

        img_input = np.full((INPUT_HEIGHT, INPUT_WIDTH, 3), 114, dtype=np.uint8)
        top = (INPUT_HEIGHT - nh) // 2
        left = (INPUT_WIDTH - nw) // 2
        img_input[top:top+nh, left:left+nw] = img_resized

        img_input = img_input[:, :, ::-1].transpose(2, 0, 1)
        img_input = np.ascontiguousarray(img_input)
        img_input = img_input.astype(np.float32) / 255.0
        img_input = img_input[None, ...]
        return img_input, scale, left, top

    def detect_y_center(self, img_bgr):
        input_tensor, scale, pad_x, pad_y = self.preprocess(img_bgr)
        outputs = self.session.run(None, {self.input_name: input_tensor})
        
        predictions = np.squeeze(outputs[0])
        
        if predictions.ndim == 1:
            return None
            
        # YOLOv8 と YOLOv5/v7 の出力形式の違いを自動吸収
        if predictions.shape[0] < predictions.shape[1] and predictions.shape[0] in [14, 15]:
            predictions = predictions.T

        num_features = predictions.shape[1]
        
        # 7番目のクラス（MyArrow）のスコアだけを抽出
        if num_features == 15:
            scores = predictions[:, 4] * predictions[:, 5 + TARGET_CLASS_INDEX]
        elif num_features == 14:
            scores = predictions[:, 4 + TARGET_CLASS_INDEX]
        else:
            print(f"[Error] 想定外のONNX出力形式です。要素数: {num_features}")
            return None

        # 閾値以上のものをフィルタリング
        mask = scores > CONF_THRESHOLD
        valid_preds = predictions[mask]
        valid_scores = scores[mask]

        if len(valid_preds) == 0:
            return None

        # MyArrowの中で最もスコアが高いものを採用
        best_pred = valid_preds[np.argmax(valid_scores)]
        
        # Y中心座標を取得して元の1080pスケールに復元
        model_cy = best_pred[1]
        orig_cy = (model_cy - pad_y) / scale
        
        return int(orig_cy)

def main():
    parser = argparse.ArgumentParser(description="MyArrowを検知してスタッツ行を自動クロップします。")
    parser.add_argument("-i", "--input", required=True, help="入力フォルダ")
    parser.add_argument("-o", "--output", required=True, help="出力フォルダ")
    parser.add_argument("-m", "--model", required=True, help="ONNXモデル")
    args = parser.parse_args()

    if not os.path.exists(args.output):
        os.makedirs(args.output)
        print(f"Created output directory: {args.output}")

    print(f"Loading ONNX model from: {args.model} ...")
    try:
        detector = ArrowDetector(args.model)
    except Exception as e:
        print(f"Error loading model: {e}")
        return
    print("Model loaded.")

    image_paths = glob.glob(os.path.join(args.input, "*.png")) + glob.glob(os.path.join(args.input, "*.jpg"))
    if not image_paths:
        print(f"No images found in {args.input}")
        return

    print(f"Found {len(image_paths)} images. Starting cropping...")
    count = 0

    for img_path in image_paths:
        filename = os.path.basename(img_path)
        
        # --- 日本語パス対応の読み込み ---
        try:
            img_array = np.fromfile(img_path, dtype=np.uint8)
            img = cv2.imdecode(img_array, cv2.IMREAD_COLOR)
        except Exception as e:
            print(f"Failed to load (Error): {filename}")
            continue
            
        if img is None:
            print(f"Failed to decode: {filename}")
            continue
        # --------------------------------

        h, w = img.shape[:2]
        if h != 1080 or w != 1920:
             scale = 1920 / w
             img = cv2.resize(img, (1920, int(h * scale)))

        y_center = detector.detect_y_center(img)

        if y_center is None:
            print(f"[Skip] MyArrow not detected in: {filename}")
            continue

        # クロップ処理 (上下マージン70px, Xは塗りPからSPまで)
        crop_h_margin = 40 
        y1 = max(0, y_center - crop_h_margin)
        y2 = min(1080, y_center + crop_h_margin)
        x1 = 780 
        x2 = 1670 

        cropped_img = img[y1:y2, x1:x2]
        output_path = os.path.join(args.output, f"crop_{filename}")
        
        # --- 日本語パス対応の保存 ---
        ext = os.path.splitext(output_path)[1]
        result, encoded_img = cv2.imencode(ext, cropped_img)
        if result:
            with open(output_path, mode='w+b') as f:
                encoded_img.tofile(f)
        # ----------------------------
        
        count += 1
        print(f"[OK] Cropped: {filename} (Y_Center: {y_center})")

    print("-----------------------------------")
    print(f"Done. Successfully cropped {count} images.")

if __name__ == "__main__":
    main()