/// IkaVision XP — WinRT OCR エンジン
///
/// Windows.Media.Ocr を使って BGRA8 バイト列からテキストを抽出します。
/// Windows 以外のプラットフォームではスタブを提供します。
use crate::types::OcrText;
use anyhow::Result;

// ---------------------------------------------------------------------------
// Windows 実装
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub use windows_impl::*;

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use windows::{
        Globalization::Language,
        Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::Streams::{DataWriter, InMemoryRandomAccessStream},
    };

    /// BGRA8 生バイト列から OCR テキストを抽出する。
    ///
    /// # Arguments
    /// * `bgra_data` - BGRA8 フォーマットのピクセルデータ
    /// * `width`     - 画像幅 (px)
    /// * `height`    - 画像高さ (px)
    /// * `lang`      - OCR 言語コード (例: "ja-JP", "en-US")。None のとき自動選択。
    pub fn ocr_from_bgra(
        bgra_data: &[u8],
        width: u32,
        height: u32,
        lang: Option<&str>,
    ) -> Result<OcrText> {
        // ── SoftwareBitmap を構築 ──────────────────────────────────────
        let bitmap = SoftwareBitmap::Create(BitmapPixelFormat::Bgra8, width as i32, height as i32)?;

        // ピクセルデータを InMemoryRandomAccessStream 経由で書き込む
        {
            let stream = InMemoryRandomAccessStream::new()?;
            let writer = DataWriter::CreateDataWriter(&stream)?;
            writer.WriteBytes(bgra_data)?;
            writer.StoreAsync()?.get()?;
            writer.FlushAsync()?.get()?;
        }

        // SoftwareBitmap のロック＆コピー（直接バッファアクセス）
        {
            use windows::Graphics::Imaging::BitmapBuffer;
            let buffer: BitmapBuffer =
                bitmap.LockBuffer(windows::Graphics::Imaging::BitmapBufferAccessMode::Write)?;
            let reference = buffer.CreateReference()?;
            use windows::Win32::System::WinRT::IMemoryBufferByteAccess;
            let byte_access: IMemoryBufferByteAccess = reference.cast()?;
            unsafe {
                let mut ptr = std::ptr::null_mut();
                let mut capacity: u32 = 0;
                byte_access.GetBuffer(&mut ptr, &mut capacity)?;
                let slice = std::slice::from_raw_parts_mut(ptr, capacity as usize);
                let copy_len = bgra_data.len().min(slice.len());
                slice[..copy_len].copy_from_slice(&bgra_data[..copy_len]);
            }
        }

        // ── OcrEngine を生成 ─────────────────────────────────────────
        let engine = if let Some(lang_code) = lang {
            let language = Language::CreateLanguage(&windows::core::HSTRING::from(lang_code))?;
            if OcrEngine::IsLanguageSupported(&language)? {
                OcrEngine::TryCreateFromLanguage(&language)?.ok_or_else(|| {
                    anyhow::anyhow!("OcrEngine creation failed for lang: {lang_code}")
                })?
            } else {
                OcrEngine::TryCreateFromUserProfileLanguages()?
                    .ok_or_else(|| anyhow::anyhow!("OcrEngine not available"))?
            }
        } else {
            // ja-JP を試みる、だめなら en-US、それもだめならユーザー既定
            let ja = Language::CreateLanguage(&windows::core::HSTRING::from("ja-JP"))?;
            if OcrEngine::IsLanguageSupported(&ja)? {
                OcrEngine::TryCreateFromLanguage(&ja)?
                    .ok_or_else(|| anyhow::anyhow!("OcrEngine creation failed"))?
            } else {
                OcrEngine::TryCreateFromUserProfileLanguages()?
                    .ok_or_else(|| anyhow::anyhow!("OcrEngine not available"))?
            }
        };

        // ── OCR 実行 ─────────────────────────────────────────────────
        let result = engine.RecognizeAsync(&bitmap)?.get()?;
        let text = result.Text()?.to_string();

        Ok(OcrText {
            text,
            confidence: 1.0, // WinRT OCR は word-level confidence を持つが簡略化
        })
    }

    /// 画像前処理（ルール/ステージ認識用）
    ///
    /// BGRA8 データをグレースケール化し、Otsu 法で2値化する。
    /// 動く背景でも文字が際立つようにする。
    pub fn preprocess_bgra(bgra_data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let n = (width * height) as usize;
        let mut out = vec![0u8; n * 4];

        // Step 1: グレースケール → 輝度計算
        let mut gray = vec![0u8; n];
        for i in 0..n {
            let b = bgra_data[i * 4] as f32;
            let g = bgra_data[i * 4 + 1] as f32;
            let r = bgra_data[i * 4 + 2] as f32;
            // ITU-R BT.601 輝度式
            gray[i] = (0.114 * b + 0.587 * g + 0.299 * r) as u8;
        }

        // Step 2: Otsu 閾値計算
        let threshold = otsu_threshold(&gray);

        // Step 3: 2値化 → BGRA8 に戻す
        for i in 0..n {
            let v = if gray[i] >= threshold { 255u8 } else { 0u8 };
            out[i * 4] = v;
            out[i * 4 + 1] = v;
            out[i * 4 + 2] = v;
            out[i * 4 + 3] = 255;
        }

        out
    }

    /// Otsu の二値化閾値を計算する
    fn otsu_threshold(gray: &[u8]) -> u8 {
        let mut hist = [0u64; 256];
        for &p in gray {
            hist[p as usize] += 1;
        }
        let total = gray.len() as f64;
        let mut sum = 0f64;
        for i in 0..256usize {
            sum += i as f64 * hist[i] as f64;
        }

        let mut sum_b = 0f64;
        let mut w_b = 0f64;
        let mut max_var = 0f64;
        let mut threshold = 0u8;

        for i in 0..256usize {
            w_b += hist[i] as f64;
            if w_b == 0.0 {
                continue;
            }
            let w_f = total - w_b;
            if w_f == 0.0 {
                break;
            }
            sum_b += i as f64 * hist[i] as f64;
            let m_b = sum_b / w_b;
            let m_f = (sum - sum_b) / w_f;
            let var = w_b * w_f * (m_b - m_f).powi(2);
            if var > max_var {
                max_var = var;
                threshold = i as u8;
            }
        }
        threshold
    }

    /// ファイルパス (PNG/BMP) から OCR を実行する（テスト・デバッグ用）
    pub fn ocr_from_file(path: &str, lang: Option<&str>) -> Result<OcrText> {
        use std::fs;
        // PNG/BMP → BGRA8 への変換は image クレートを使わず
        // StorageFile + BitmapDecoder を使う
        use windows::{
            core::HSTRING,
            Graphics::Imaging::BitmapDecoder,
            Storage::{FileAccessMode, StorageFile},
        };

        let abs_path = std::fs::canonicalize(path)
            .map_err(|e| anyhow::anyhow!("file not found: {path}: {e}"))?;
        let path_str = abs_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid path"))?;

        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path_str))?.get()?;
        let stream = file.OpenAsync(FileAccessMode::Read)?.get()?;
        let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;

        let software_bitmap = decoder.GetSoftwareBitmapAsync()?.get()?;

        // BGRA8 に変換
        let bgra_bitmap = SoftwareBitmap::Convert(
            &software_bitmap,
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
        )?;

        let width = bgra_bitmap.PixelWidth()? as u32;
        let height = bgra_bitmap.PixelHeight()? as u32;

        // バイト列取得
        let engine = build_engine(lang)?;
        let result = engine.RecognizeAsync(&bgra_bitmap)?.get()?;
        let text = result.Text()?.to_string();
        let _ = (width, height); // suppress warning

        Ok(OcrText {
            text,
            confidence: 1.0,
        })
    }

    fn build_engine(lang: Option<&str>) -> Result<OcrEngine> {
        if let Some(lang_code) = lang {
            let language = Language::CreateLanguage(&windows::core::HSTRING::from(lang_code))?;
            if OcrEngine::IsLanguageSupported(&language)? {
                return Ok(OcrEngine::TryCreateFromLanguage(&language)?
                    .ok_or_else(|| anyhow::anyhow!("OcrEngine creation failed"))?);
            }
        }
        let ja = Language::CreateLanguage(&windows::core::HSTRING::from("ja-JP"))?;
        if OcrEngine::IsLanguageSupported(&ja)? {
            return Ok(OcrEngine::TryCreateFromLanguage(&ja)?
                .ok_or_else(|| anyhow::anyhow!("OcrEngine creation failed"))?);
        }
        OcrEngine::TryCreateFromUserProfileLanguages()?
            .ok_or_else(|| anyhow::anyhow!("OcrEngine not available"))
    }
}

// ---------------------------------------------------------------------------
// 非 Windows スタブ
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
pub fn ocr_from_bgra(
    _bgra_data: &[u8],
    _width: u32,
    _height: u32,
    _lang: Option<&str>,
) -> Result<OcrText> {
    Err(anyhow::anyhow!("WinRT OCR is only available on Windows"))
}

#[cfg(not(target_os = "windows"))]
pub fn ocr_from_file(_path: &str, _lang: Option<&str>) -> Result<OcrText> {
    Err(anyhow::anyhow!("WinRT OCR is only available on Windows"))
}

#[cfg(not(target_os = "windows"))]
pub fn preprocess_bgra(bgra_data: &[u8], _width: u32, _height: u32) -> Vec<u8> {
    bgra_data.to_vec()
}

// ---------------------------------------------------------------------------
// 単体テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_otsu_does_not_panic() {
        // 4×4 の真っ白 BGRA 画像でパニックしないことを確認
        let data = vec![255u8; 4 * 4 * 4];
        let _out = preprocess_bgra(&data, 4, 4);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_ocr_engine_available() {
        use windows::{Globalization::Language, Media::Ocr::OcrEngine};
        let lang = Language::CreateLanguage(&windows::core::HSTRING::from("en-US")).unwrap();
        let supported = OcrEngine::IsLanguageSupported(&lang).unwrap_or(false);
        // en-US は常にサポートされているはず
        assert!(supported, "en-US OCR should be available on Windows");
    }
}
