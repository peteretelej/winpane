//! Proof-of-concept: DirectComposition transparent window with Direct2D rendering.
//!
//! Creates a floating colored circle on the desktop with per-pixel transparency.
//! This proves the DirectComposition rendering pipeline end-to-end:
//!
//!   D3D11 device -> DXGI swap chain (premultiplied alpha, for composition)
//!   -> DirectComposition visual tree bound to HWND
//!   -> Direct2D render target on swap chain surface
//!
//! Run on Windows: cargo run -p winpane-core --example hello_transparent

use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Graphics::Direct2D::Common::*,
    Win32::Graphics::Direct2D::*,
    Win32::Graphics::Direct3D::*,
    Win32::Graphics::Direct3D11::*,
    Win32::Graphics::DirectComposition::*,
    Win32::Graphics::Dxgi::Common::*,
    Win32::Graphics::Dxgi::*,
    Win32::System::Com::*,
    Win32::System::LibraryLoader::*,
    Win32::UI::WindowsAndMessaging::*,
};

const WINDOW_SIZE: i32 = 400;
const CIRCLE_RADIUS: f32 = 150.0;

fn main() -> Result<()> {
    unsafe {
        // COM initialization (required for DirectComposition and Direct2D)
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        // 1. Create a transparent popup window
        let hwnd = create_window()?;

        // 2. Create D3D11 device (hardware, WARP fallback)
        let d3d_device = create_d3d11_device()?;
        let dxgi_device: IDXGIDevice = d3d_device.cast()?;

        // 3. Create DXGI swap chain for composition (not for HWND - that's the key difference)
        let swapchain = create_composition_swapchain(&d3d_device)?;

        // 4. DirectComposition: bind swap chain -> visual -> target -> HWND
        let dcomp_device: IDCompositionDevice = DCompositionCreateDevice(&dxgi_device)?;
        let dcomp_target = dcomp_device.CreateTargetForHwnd(hwnd, true)?;
        let visual = dcomp_device.CreateVisual()?;
        visual.SetContent(&swapchain)?;
        dcomp_target.SetRoot(&visual)?;

        // 5. Create Direct2D device context for drawing
        let d2d_factory: ID2D1Factory1 =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let d2d_device = d2d_factory.CreateDevice(&dxgi_device)?;
        let dc = d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

        // 6. Create bitmap target from swap chain back buffer
        let surface: IDXGISurface = swapchain.GetBuffer(0)?;
        let bitmap = dc.CreateBitmapFromDxgiSurface(
            &surface,
            Some(&D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                ..Default::default()
            }),
        )?;
        dc.SetTarget(&bitmap);

        // 7. Draw a colored circle on a fully transparent background
        dc.BeginDraw();
        dc.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        let brush = dc.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r: 0.2,
                g: 0.6,
                b: 1.0,
                a: 0.85,
            },
            None,
        )?;

        let center = WINDOW_SIZE as f32 / 2.0;
        dc.FillEllipse(
            &D2D1_ELLIPSE {
                point: D2D_POINT_2F {
                    x: center,
                    y: center,
                },
                radiusX: CIRCLE_RADIUS,
                radiusY: CIRCLE_RADIUS,
            },
            &brush,
        );

        dc.EndDraw(None, None)?;

        // 8. Present the swap chain and commit the DirectComposition tree
        swapchain.Present(1, DXGI_PRESENT(0)).ok()?;
        dcomp_device.Commit()?;

        // 9. Show the window and run the message loop
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        println!("winpane hello_transparent: floating circle should be visible on desktop.");
        println!("Close this console or press Ctrl+C to exit.");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            DispatchMessageW(&msg);
        }

        Ok(())
    }
}

/// Creates a popup window with DirectComposition-compatible extended styles.
///
/// - `WS_EX_NOREDIRECTIONBITMAP`: Required for DirectComposition (no DWM redirection surface).
/// - `WS_EX_TOPMOST`: Always on top of other windows.
/// - `WS_EX_TOOLWINDOW`: Excluded from the taskbar and Alt+Tab.
/// - `WS_EX_NOACTIVATE`: Never receives keyboard focus.
/// - `WS_POPUP`: No window chrome (caption, border, etc).
unsafe fn create_window() -> Result<HWND> {
    let instance = GetModuleHandleW(None)?;
    let class_name = w!("winpane_hello");

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(wndproc),
        hInstance: instance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    RegisterClassExW(&wc);

    CreateWindowExW(
        WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        class_name,
        w!("winpane hello"),
        WS_POPUP,
        100,
        100,
        WINDOW_SIZE,
        WINDOW_SIZE,
        None,
        None,
        Some(instance.into()),
        None,
    )
}

/// Creates a D3D11 device with BGRA support (required for Direct2D interop).
/// Tries hardware acceleration first, falls back to the WARP software rasterizer.
unsafe fn create_d3d11_device() -> Result<ID3D11Device> {
    let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
    let mut device = None;

    let result = D3D11CreateDevice(
        None,
        D3D_DRIVER_TYPE_HARDWARE,
        HMODULE::default(),
        flags,
        None,
        D3D11_SDK_VERSION,
        Some(&mut device),
        None,
        None,
    );

    if result.is_ok() {
        return Ok(device.unwrap());
    }

    // CI runners and VMs may lack a GPU - fall back to WARP
    D3D11CreateDevice(
        None,
        D3D_DRIVER_TYPE_WARP,
        HMODULE::default(),
        flags,
        None,
        D3D11_SDK_VERSION,
        Some(&mut device),
        None,
        None,
    )?;

    Ok(device.unwrap())
}

/// Creates a DXGI swap chain for DirectComposition (not for HWND).
///
/// Key difference from `CreateSwapChainForHwnd`: this swap chain is bound to a
/// DirectComposition visual instead of directly to a window. This enables
/// `DXGI_ALPHA_MODE_PREMULTIPLIED` for per-pixel transparency.
unsafe fn create_composition_swapchain(device: &ID3D11Device) -> Result<IDXGISwapChain1> {
    let dxgi_device: IDXGIDevice = device.cast()?;
    let adapter = dxgi_device.GetAdapter()?;
    let factory: IDXGIFactory2 = adapter.GetParent()?;

    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: WINDOW_SIZE as u32,
        Height: WINDOW_SIZE as u32,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
        ..Default::default()
    };

    factory.CreateSwapChainForComposition(device, &desc, None)
}

/// Window procedure: click-through and close handling.
extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            // HTTRANSPARENT (-1): all mouse input passes through to the window below
            WM_NCHITTEST => LRESULT(-1),
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
