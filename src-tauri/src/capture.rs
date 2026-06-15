/// InkGraph — 画面キャプチャ (Windows Graphics Capture)
///
/// `WindowCaptureSession` を一度だけ作成し、フレームを繰り返し取得する。
/// セッションを毎フレーム作り直すとキャプチャインジケーターが点滅するため、
/// セッションをループ全体で保持する設計にしている。

use anyhow::Result;
use crate::types::WindowInfo;

// ---------------------------------------------------------------------------
// Windows 実装
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub use windows_impl::*;

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    use windows::{
        core::{IInspectable, Interface},
        Foundation::TypedEventHandler,
        Graphics::{
            Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession},
            DirectX::{
                Direct3D11::IDirect3DDevice,
                DirectXPixelFormat,
            },
        },
        Win32::{
            Foundation::{BOOL, HWND, LPARAM},
            Graphics::{
                Direct3D::D3D_DRIVER_TYPE_HARDWARE,
                Direct3D11::{
                    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
                    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION,
                    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
                },
                Dxgi::IDXGIDevice,
            },
            System::WinRT::{
                Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess},
                Graphics::Capture::IGraphicsCaptureItemInterop,
            },
            UI::WindowsAndMessaging::{
                EnumWindows, GetParent, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW,
                IsWindowVisible, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
            },
        },
    };

    // -------------------------------------------------------------------------
    // ウィンドウ列挙
    // -------------------------------------------------------------------------

    pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
        let mut windows: Vec<WindowInfo> = Vec::new();
        let ptr = &mut windows as *mut Vec<WindowInfo> as isize;
        unsafe {
            EnumWindows(Some(enum_window_proc), LPARAM(ptr))?;
        }
        Ok(windows)
    }

    extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            // 非表示ウィンドウを除外
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }

            // 子ウィンドウを除外 (親がある = 別ウィンドウの一部)
            // GetParent は Result<HWND> を返す: Ok(null) = 親なし, Ok(非null) = 子ウィンドウ
            if matches!(GetParent(hwnd), Ok(p) if !p.0.is_null()) {
                return BOOL(1);
            }

            // ツールウィンドウを除外 (システムトレイ・オーバーレイ等)
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
                return BOOL(1);
            }

            // タイトルが短すぎるウィンドウを除外
            let len = GetWindowTextLengthW(hwnd);
            if len < 2 {
                return BOOL(1);
            }
            let mut buf = vec![0u16; (len + 1) as usize];
            GetWindowTextW(hwnd, &mut buf);
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            if title.trim().is_empty() {
                return BOOL(1);
            }

            let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);
            windows.push(WindowInfo { hwnd: hwnd.0 as u64, title });
        }
        BOOL(1)
    }

    // -------------------------------------------------------------------------
    // 永続キャプチャセッション
    // -------------------------------------------------------------------------

    /// D3D11 デバイスと WGC セッションを保持する。
    /// ループ開始時に一度だけ作成し、フレームごとに `get_frame()` を呼ぶ。
    pub struct WindowCaptureSession {
        d3d_device:    ID3D11Device,
        d3d_context:   ID3D11DeviceContext,
        frame_pool:    Direct3D11CaptureFramePool,
        session:       GraphicsCaptureSession,
        /// FrameArrived イベントが発火したことを示すフラグ
        frame_arrived: Arc<AtomicBool>,
    }

    impl WindowCaptureSession {
        pub fn new(hwnd_val: u64) -> Result<Self> {
            let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);

            // ── D3D11 デバイス ────────────────────────────────────────────
            let mut d3d_device:  Option<ID3D11Device>        = None;
            let mut d3d_context: Option<ID3D11DeviceContext> = None;
            unsafe {
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    None,
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut d3d_device),
                    None,
                    Some(&mut d3d_context),
                )?;
            }
            let d3d_device  = d3d_device .ok_or_else(|| anyhow::anyhow!("D3D11 device failed"))?;
            let d3d_context = d3d_context.ok_or_else(|| anyhow::anyhow!("D3D11 context failed"))?;

            // ── IDXGIDevice → WinRT IDirect3DDevice ──────────────────────
            let dxgi_device: IDXGIDevice = d3d_device.cast()?;
            let inspectable: IInspectable =
                unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
            let winrt_device: IDirect3DDevice = inspectable.cast()?;

            // ── GraphicsCaptureItem ───────────────────────────────────────
            let interop: IGraphicsCaptureItemInterop =
                windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd)? };
            let size = item.Size()?;

            // ── フレームプール & セッション ───────────────────────────────
            let frame_pool = Direct3D11CaptureFramePool::Create(
                &winrt_device,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                size,
            )?;

            // FrameArrived イベントでフラグを立てる (ポーリングより確実)
            let frame_arrived = Arc::new(AtomicBool::new(false));
            let flag = frame_arrived.clone();
            frame_pool.FrameArrived(&TypedEventHandler::new(move |_, _| {
                flag.store(true, Ordering::Relaxed);
                Ok(())
            }))?;

            let session = frame_pool.CreateCaptureSession(&item)?;
            session.StartCapture()?;
            log::info!("[capture] WGC session started (hwnd={hwnd_val})");

            Ok(Self { d3d_device, d3d_context, frame_pool, session, frame_arrived })
        }

        /// 最新フレームを BGRA8 として取得する。
        ///
        /// # 取得戦略 (2フェーズ)
        ///
        /// 1. まず `TryGetNextFrame` を直接ポーリング (500ms)。
        ///    静止した画面でも WGC のバッファにフレームが残っている場合はすぐ取得できる。
        ///
        /// 2. バッファが空なら `FrameArrived` フラグが立つまで最大 3 秒待機する。
        ///    注意: `FrameArrived` は Tokio worker スレッドの中で待つとデッドロックする
        ///    ことがある (同一スレッドへのコールバック競合)。このメソッドは
        ///    `std::thread::spawn` で生成した専用スレッドからのみ呼ぶこと。
        pub fn get_frame(&self) -> Result<CapturedFrame> {
            use std::time::{Duration, Instant};

            // フェーズ 1: バッファに既存フレームがあれば即取得 (500ms 以内)
            let phase1_deadline = Instant::now() + Duration::from_millis(500);
            let maybe_frame = loop {
                if let Ok(f) = self.frame_pool.TryGetNextFrame() {
                    break Some(f);
                }
                if Instant::now() > phase1_deadline {
                    break None;
                }
                std::thread::sleep(Duration::from_millis(5));
            };

            let wgc_frame = if let Some(f) = maybe_frame {
                f
            } else {
                // フェーズ 2: FrameArrived イベントを最大 3 秒待機
                // (専用スレッドで呼ばれているので Tokio との競合はない)
                let deadline = Instant::now() + Duration::from_secs(3);
                loop {
                    if self.frame_arrived.swap(false, Ordering::Relaxed) {
                        match self.frame_pool.TryGetNextFrame() {
                            Ok(f) => break f,
                            Err(e) => anyhow::bail!("TryGetNextFrame failed after FrameArrived: {e}"),
                        }
                    }
                    if Instant::now() > deadline {
                        anyhow::bail!(
                            "get_frame timed out (FrameArrived イベントが 3 秒以内に発火しなかった。\
                             hwnd が無効か、ウィンドウが最小化されている可能性がある)"
                        );
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            };

            let surface = wgc_frame.Surface()?;
            let (bgra, width, height) =
                surface_to_bgra8(&self.d3d_device, &surface, &self.d3d_context)?;
            Ok(CapturedFrame { bgra, width, height })
        }
    }

    impl Drop for WindowCaptureSession {
        fn drop(&mut self) {
            let _ = self.session.Close();
            let _ = self.frame_pool.Close();
            log::info!("[capture] WGC session closed");
        }
    }

    // -------------------------------------------------------------------------
    // 内部ヘルパー
    // -------------------------------------------------------------------------

    fn surface_to_bgra8(
        device:  &ID3D11Device,
        surface: &windows::Graphics::DirectX::Direct3D11::IDirect3DSurface,
        context: &ID3D11DeviceContext,
    ) -> Result<(Vec<u8>, u32, u32)> {
        let interop: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        let texture: ID3D11Texture2D = unsafe { interop.GetInterface()? };

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut desc) };
        let width  = desc.Width;
        let height = desc.Height;

        let staging_desc = D3D11_TEXTURE2D_DESC {
            Usage:          D3D11_USAGE_STAGING,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            BindFlags:      0,
            MiscFlags:      0,
            ..desc
        };
        let mut staging_tex: Option<ID3D11Texture2D> = None;
        unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut staging_tex))? };
        let staging_tex = staging_tex.unwrap();

        unsafe {
            context.CopyResource(&staging_tex, &texture);
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            context.Map(&staging_tex, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

            let row_pitch = mapped.RowPitch as usize;
            let mut bgra = Vec::with_capacity((width * height * 4) as usize);
            let src = mapped.pData as *const u8;
            for row in 0..height as usize {
                let start = row * row_pitch;
                let end   = start + width as usize * 4;
                bgra.extend_from_slice(std::slice::from_raw_parts(src.add(start), end - start));
            }
            context.Unmap(&staging_tex, 0);
            Ok((bgra, width, height))
        }
    }
}

// ---------------------------------------------------------------------------
// 非 Windows スタブ
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    Err(anyhow::anyhow!("capture is only available on Windows"))
}

// ---------------------------------------------------------------------------
// 共通型
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub bgra:   Vec<u8>,
    pub width:  u32,
    pub height: u32,
}

// ---------------------------------------------------------------------------
// ファイルから CapturedFrame を生成 (デバッグ・テスト用、全プラットフォーム共通)
// ---------------------------------------------------------------------------

/// PNG / JPEG / BMP 等の画像ファイルを読み込んで CapturedFrame (BGRA8) に変換する。
/// WGC を使わないため、キャプチャカード画像や CI 上のテスト PNG を直接渡せる。
pub fn frame_from_file(path: &str) -> Result<CapturedFrame> {
    let img = image::open(path)
        .map_err(|e| anyhow::anyhow!("画像ファイルの読み込み失敗 ({path}): {e}"))?
        .into_rgba8(); // RGBA8 として読む
    let width  = img.width();
    let height = img.height();
    // image crate は RGBA 順なので BGRA に変換する
    let bgra: Vec<u8> = img
        .pixels()
        .flat_map(|p| [p[2], p[1], p[0], p[3]]) // R,G,B,A → B,G,R,A
        .collect();
    Ok(CapturedFrame { bgra, width, height })
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// frame_from_file は存在しないファイルに対してエラーを返す。
    /// (クロスプラットフォーム; Ubuntu CI でも実行可能)
    #[test]
    fn frame_from_file_missing_returns_error() {
        let result = frame_from_file("/nonexistent/path/frame.png");
        assert!(result.is_err(), "存在しないパスはエラーになるはず");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("画像ファイルの読み込み失敗"),
            "エラーメッセージが期待通りでない: {msg}"
        );
    }

    /// frame_from_file は PNG を正しく BGRA8 に変換する。
    /// 2×1 ピクセルの最小 PNG をメモリ上で作成して検証する。
    #[test]
    fn frame_from_file_bgra_conversion() {
        // 2×1 ピクセル (赤 + 透明) の PNG を一時ファイルに書く
        // image crate で PNG エンコード
        let mut img = image::RgbaImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255])); // 赤 (R=255, G=0, B=0, A=255)
        img.put_pixel(1, 0, image::Rgba([0, 128, 64, 200])); // 緑がかった色

        let tmp = std::env::temp_dir().join("inkgraph_test_frame.png");
        img.save(&tmp).expect("テスト用 PNG の保存に失敗");

        let frame = frame_from_file(tmp.to_str().unwrap())
            .expect("PNG の読み込みに失敗");

        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 1);
        assert_eq!(frame.bgra.len(), 2 * 1 * 4); // 各ピクセル 4 バイト

        // ピクセル 0: R=255, G=0, B=0, A=255 → BGRA = [0, 0, 255, 255]
        assert_eq!(&frame.bgra[0..4], &[0u8, 0, 255, 255], "ピクセル0の BGRA 変換が不正");

        // ピクセル 1: R=0, G=128, B=64, A=200 → BGRA = [64, 128, 0, 200]
        assert_eq!(&frame.bgra[4..8], &[64u8, 128, 0, 200], "ピクセル1の BGRA 変換が不正");

        let _ = std::fs::remove_file(&tmp);
    }
}
