import argparse
import glob
import os
import ast
import cv2
import numpy as np
import onnxruntime as ort

# ==========================================
# 固定の設定項目
# ==========================================
# IDではなくラベル名（文字列）で指定します
TARGET_CLASS_NAME = "MyArrow"  

MODEL_INPUT_SIZE = (640, 640)  # モデルの入力サイズ

# 基準となる解像度（1920x1080）
REF_W = 1920
REF_H = 1080

# 1920x1080の画像におけるクロップ座標 [y1:y2, x1:x2]
REF_CROP_Y1 = 20
REF_CROP_Y2 = 180
REF_CROP_X1 = 780
REF_CROP_X2 = 1850
# ==========================================


def parse_args():
    """コマンドライン引数の設定"""
    parser = argparse.ArgumentParser(
        description="ONNXモデルで指定ラベルを検出した画像のみ特定領域をクロップするスクリプト"
    )
    parser.add_argument(
        "--model", "-m", type=str, required=True, help="ONNXモデルファイルのパス"
    )
    parser.add_argument(
        "--input", "-i", type=str, required=True, help="入力画像が格納されているディレクトリのパス"
    )
    parser.add_argument(
        "--output", "-o", type=str, required=True, help="クロップ画像の保存先ディレクトリのパス"
    )
    parser.add_argument(
        "--conf", "-c", type=float, default=0.5, help="検出の信頼度しきい値（デフォルト: 0.5）"
    )
    return parser.parse_args()


def preprocess_image(image, input_size):
    """ONNXモデル用の画像前処理"""
    img_rgb = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
    img_resized = cv2.resize(img_rgb, input_size)
    img_normalized = img_resized.astype(np.float32) / 255.0
    img_chw = np.transpose(img_normalized, (2, 0, 1))
    img_batch = np.expand_dims(img_chw, axis=0)
    return img_batch


def get_class_names_from_onnx(session):
    """
    ONNXモデルのメタデータからクラス名の辞書またはリストを取得する関数。
    """
    meta = session.get_modelmeta()
    custom_meta = meta.custom_metadata_map

    if 'names' in custom_meta:
        try:
            names_dict = ast.literal_eval(custom_meta['names'])
            return names_dict
        except Exception as e:
            print(f"警告: メタデータ 'names' のパースに失敗しました: {e}")
    
    return None


def detect_target_class(outputs, target_id, conf_thresh):
    """
    推論結果からターゲットIDが含まれているか判定する（YOLOv8/v11対応版）
    """
    output = outputs[0]  # 通常、最初の出力が推論結果
    
    # YOLOv8/v11の一般的な出力形状 [1, 4 + num_classes, num_anchors]
    if len(output.shape) == 3 and output.shape[1] < output.shape[2]:
        # [1, num_anchors, 4 + num_classes] に転置
        output = np.transpose(output, (0, 2, 1))

    # バッチの0番目を取得 (shape: [num_anchors, 4 + num_classes])
    predictions = output[0]
    
    # ターゲットクラスの最高スコアを記録する変数
    max_confidence = 0.0

    for pred in predictions:
        # pred[0:4] はボックス座標 (cx, cy, w, h)
        # pred[4:] が各クラスのスコア
        scores = pred[4:] 
        
        if len(scores) == 0:
            continue
            
        class_id = np.argmax(scores)
        confidence = scores[class_id]

        if class_id == target_id and confidence >= conf_thresh:
            if confidence > max_confidence:
                max_confidence = confidence

    if max_confidence > 0:
        return True, max_confidence

    return False, 0.0


def main():
    args = parse_args()
    os.makedirs(args.output, exist_ok=True)

    # 1. ONNXセッションの開始
    print(f"モデルを読み込み中: {args.model}")
    try:
        session = ort.InferenceSession(args.model, providers=["CPUExecutionProvider"])
        input_name = session.get_inputs()[0].name
    except Exception as e:
        print(f"モデルの読み込みに失敗しました: {e}")
        return

    # 2. モデルからクラス名を取得してIDを特定
    class_names = get_class_names_from_onnx(session)
    target_class_id = None

    if class_names is not None:
        print(f"モデル内のクラス情報が見つかりました: {class_names}")
        for class_id, class_name in class_names.items():
            if class_name == TARGET_CLASS_NAME:
                target_class_id = int(class_id)
                break
                
        if target_class_id is None:
            print(f"エラー: モデル内に '{TARGET_CLASS_NAME}' というクラス名が見つかりませんでした。")
            return
    else:
        print("警告: ONNXファイルからクラス名のメタデータを取得できませんでした。")
        return

    print(f"ターゲット '{TARGET_CLASS_NAME}' は クラスID [{target_class_id}] として処理します。")
    print("-" * 40)

    # 3. 入力ディレクトリから画像ファイルを取得
    image_extensions = ["*.jpg", "*.jpeg", "*.png", "*.JPG", "*.JPEG", "*.PNG"]
    image_paths = []
    for ext in image_extensions:
        image_paths.extend(glob.glob(os.path.join(args.input, ext)))

    if not image_paths:
        print(f"入力ディレクトリに画像が見つかりませんでした: {args.input}")
        return

    print(f"{len(image_paths)} 枚の画像を処理します...")

    # 4. 各画像をループ処理
    for img_path in image_paths:
        filename = os.path.basename(img_path)
        
        # 日本語パス対応の読み込み
        try:
            nparr = np.fromfile(img_path, np.uint8)
            original_image = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
        except Exception:
            original_image = None

        if original_image is None:
            print(f"[{filename}] 画像を読み込めませんでした。スキップします。")
            continue

        # 前処理と推論
        input_tensor = preprocess_image(original_image, MODEL_INPUT_SIZE)
        outputs = session.run(None, {input_name: input_tensor})

        # 判定
        is_detected, confidence = detect_target_class(outputs, target_class_id, args.conf)

        if is_detected:
            print(f"[{filename}] {TARGET_CLASS_NAME}を検出 (信頼度: {confidence:.2f}) -> クロップ中...")
            
            # ------------------------------------------
            # 画像サイズに合わせて動的にクロップ座標を計算
            # ------------------------------------------
            img_h, img_w = original_image.shape[:2]
            
            crop_y1 = int(REF_CROP_Y1 * (img_h / REF_H))
            crop_y2 = int(REF_CROP_Y2 * (img_h / REF_H))
            crop_x1 = int(REF_CROP_X1 * (img_w / REF_W))
            crop_x2 = int(REF_CROP_X2 * (img_w / REF_W))

            cropped_image = original_image[crop_y1:crop_y2, crop_x1:crop_x2]

            # 日本語パス対応の保存
            output_path = os.path.join(args.output, f"cropped_{filename}")
            ext = os.path.splitext(output_path)[1]
            if not ext:
                ext = ".jpg"
                
            result, encoded_img = cv2.imencode(ext, cropped_image)
            if result:
                with open(output_path, mode='w+b') as f:
                    encoded_img.tofile(f)
            else:
                print(f"[{filename}] 画像の保存（エンコード）に失敗しました。")
        else:
            print(f"[{filename}] {TARGET_CLASS_NAME}未検出 -> スキップ")

    print("すべての処理が完了しました。")


if __name__ == "__main__":
    main()