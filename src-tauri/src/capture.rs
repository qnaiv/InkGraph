/// IkaVision XP — 画面キャプチャ (Windows Graphics Capture)
///
/// WGC (Windows.Graphics.Capture) を使ってウィンドウフレームを取得します。
/// フレームは BGRA8 バイト列として返します。

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
    use windows::{
        core::Interface,
        Win32::{
            Foundation::{BOOL, HWND, LPARAM},
            UI::WindowsAndMessaging::{
                EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
            },
        },
    };

    // -------------------------------------------------------------------------
    // ウィンドウ列挙
    // -------------------------------------------------------------------------

    /// キャプチャ可能なウィンドウの一覧を返す
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

            // タイトル取得
            let len = GetWindowTextLengthW(hwnd);
            if len == 0 {
                return BOOL(1);
            }
            let mut buf = vec![0u16; (len + 1) as usize];
            GetWindowTextW(hwnd, &mut buf);
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            if title.trim().is_empty() {
                return BOOL(1);
            }

            let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);
            windows.push(WindowInfo {
                hwnd: hwnd.0 as u64,
                title,
            });
        }
        BOOL(1)
    }

    // -------------------------------------------------------------------------
    // フレームキャプチャ
    // -------------------------------------------------------------------------

    /// 指定 HWND のウィンドウから1フレームを BGRA8 で取得する。
    ///
    /// 非同期 WGC API を同期的にラップしています。
    /// 本格運用では専用スレッドに移動し、チャネル経由で送信してください。
    pub fn capture_window_frame(hwnd_val: u64) -> Result<CapturedFrame> {
        use windows::{
            Graphics::{
                Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem},
                DirectX::DirectXPixelFormat,
            },
            Win32::{
                Foundation::HWND,
                Graphics::{
                    Direct3D::D3D_DRIVER_TYPE_HARDWARE,
                    Direct3D11::{
                        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext,
                        D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
                    },
                    Dxgi::IDXGIDevice,
                },
                System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
            },
        };

        let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);

        // ── D3D11 デバイス作成 ────────────────────────────────────────
        let mut d3d_device: Option<ID3D11Device> = None;
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
        let d3d_device = d3d_device.ok_or_else(|| anyhow::anyhow!("D3D11 device creation failed"))?;

        // IDXGIDevice → IDirect3DDevice (WinRT)
        let dxgi_device: IDXGIDevice = d3d_device.cast()?;
        let direct3d_device = create_direct3d_device(&dxgi_device)?;

        // ── GraphicsCaptureItem を HWND から作成 ──────────────────────
        let interop: IGraphicsCaptureItemInterop = windows::core::factory::<
            GraphicsCaptureItem,
            IGraphicsCaptureItemInterop,
        >()?;
        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd)? };
        let size = item.Size()?;

        // ── フレームプール & セッション ────────────────────────────────
        let frame_pool = Direct3D11CaptureFramePool::Create(
            &direct3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            size,
        )?;
        let session = frame_pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;

        // ── フレーム取得 (最大 100ms 待機) ──────────────────────────
        use std::time::{Duration, Instant};
        let deadline = Instant::now() + Duration::from_millis(100);
        let frame = loop {
            if let Ok(f) = frame_pool.TryGetNextFrame() {
                break f;
            }
            if Instant::now() > deadline {
                anyhow::bail!("frame capture timed out");
            }
            std::thread::sleep(Duration::from_millis(5));
        };

        let surface = frame.Surface()?;

        // ── テクスチャを CPU 側にコピーして BGRA8 取得 ──────────────
        let (bgra, width, height) = surface_to_bgra8(&d3d_device, &surface, &d3d_context.unwrap())?;

        session.Close()?;
        frame_pool.Close()?;

        Ok(CapturedFrame { bgra, width, height })
    }

    /// IDirect3DSurface を BGRA8 バイト列に変換する
    fn surface_to_bgra8(
        device: &windows::Win32::Graphics::Direct3D11::ID3D11Device,
        surface: &windows::Graphics::DirectX::Direct3D11::IDirect3DSurface,
        context: &windows::Win32::Graphics::Direct3D11::ID3D11DeviceContext,
    ) -> Result<(Vec<u8>, u32, u32)> {
        use windows::Win32::Graphics::Direct3D11::{
            ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE,
            D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
        };

        // IDirect3DSurface → ID3D11Texture2D
        let interop: windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess =
            surface.cast()?;
        let texture: ID3D11Texture2D = unsafe { interop.GetInterface()? };

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut desc) };
        let width = desc.Width;
        let height = desc.Height;

        // ステージングテクスチャを作成して CPU 読み出し
        let staging_desc = D3D11_TEXTURE2D_DESC {
            Usage: D3D11_USAGE_STAGING,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            BindFlags: 0,
            MiscFlags: 0,
            ..desc
        };
        let mut staging_tex: Option<ID3D11Texture2D> = None;
        unsafe {
            device.CreateTexture2D(&staging_desc, None, Some(&mut staging_tex))?;
        }
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
                let end = start + (width as usize * 4);
                bgra.extend_from_slice(std::slice::from_raw_parts(src.add(start), end - start));
            }
            context.Unmap(&staging_tex, 0);
            Ok((bgra, width, height))
        }
    }

    /// IDXGIDevice から WinRT IDirect3DDevice を作成する
    fn create_direct3d_device(
        dxgi_device: &windows::Win32::Graphics::Dxgi::IDXGIDevice,
    ) -> Result<windows::Graphics::DirectX::Direct3D11::IDirect3DDevice> {
        use windows::{
            core::IInspectable,
            Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
        };
        let inspectable: IInspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(dxgi_device)? };
        Ok(inspectable.cast()?)
    }
}

// ---------------------------------------------------------------------------
// 非 Windows スタブ
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
pub fn list_capturable_windows() -> Result<Vec<WindowInfo>> {
    Err(anyhow::anyhow!("capture is only available on Windows"))
}

#[cfg(not(target_os = "windows"))]
pub fn capture_window_frame(_hwnd_val: u64) -> Result<CapturedFrame> {
    Err(anyhow::anyhow!("capture is only available on Windows"))
}

// ---------------------------------------------------------------------------
// 共通型
// ---------------------------------------------------------------------------

/// キャプチャしたフレームデータ
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub bgra: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
