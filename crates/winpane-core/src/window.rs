use std::cell::RefCell;

use windows::{
    core::*, Win32::Foundation::*, Win32::System::LibraryLoader::*, Win32::UI::HiDpi::*,
    Win32::UI::WindowsAndMessaging::*,
};

use crate::types::Error;

// --- DPI change queue (thread-local, same thread as message loop) ---

pub(crate) struct DpiChangeEvent {
    pub hwnd: HWND,
    pub new_dpi: u32,
}

thread_local! {
    pub(crate) static PENDING_DPI_CHANGES: RefCell<Vec<DpiChangeEvent>> = RefCell::new(Vec::new());
}

// --- SendHwnd wrapper ---

/// Wrapper around HWND that implements Send.
/// Safety: PostMessage to an HWND is thread-safe by Win32 specification.
#[derive(Clone, Copy)]
pub struct SendHwnd(pub HWND);
unsafe impl Send for SendHwnd {}

// --- Window class registration ---

static REGISTER_CLASSES: std::sync::Once = std::sync::Once::new();

pub(crate) unsafe fn ensure_classes_registered() {
    REGISTER_CLASSES.call_once(|| {
        let instance = GetModuleHandleW(None).unwrap();

        let hud_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(hud_wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("winpane_hud"),
            ..Default::default()
        };
        RegisterClassExW(&hud_class);

        let ctrl_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(control_wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("winpane_control"),
            ..Default::default()
        };
        RegisterClassExW(&ctrl_class);
    });
}

// --- Window creation ---

pub(crate) unsafe fn create_hud_window(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<HWND, Error> {
    CreateWindowExW(
        WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        w!("winpane_hud"),
        w!("winpane"),
        WS_POPUP,
        x,
        y,
        width as i32,
        height as i32,
        None,
        None,
        Some(GetModuleHandleW(None).unwrap().into()),
        None,
    )
    .map_err(|e| Error::WindowCreation(e.to_string()))
}

pub(crate) unsafe fn create_control_window() -> Result<HWND, Error> {
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("winpane_control"),
        w!("winpane_ctrl"),
        WS_POPUP,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        None,
        Some(GetModuleHandleW(None).unwrap().into()),
        None,
    )
    .map_err(|e| Error::WindowCreation(e.to_string()))
}

// --- Window procedures ---

extern "system" fn hud_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_NCHITTEST => LRESULT(-1), // HTTRANSPARENT - click through

            WM_DPICHANGED => {
                let new_dpi = (wparam.0 & 0xFFFF) as u32;
                let suggested_rect = *(lparam.0 as *const RECT);

                let _ = SetWindowPos(
                    hwnd,
                    None,
                    suggested_rect.left,
                    suggested_rect.top,
                    suggested_rect.right - suggested_rect.left,
                    suggested_rect.bottom - suggested_rect.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );

                PENDING_DPI_CHANGES.with(|changes| {
                    changes.borrow_mut().push(DpiChangeEvent { hwnd, new_dpi });
                });

                LRESULT(0)
            }

            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

extern "system" fn control_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// --- DPI utilities ---

/// Returns the DPI scale factor for a window (1.0 at 96 DPI, 1.5 at 144 DPI, etc.)
pub(crate) unsafe fn get_dpi_scale(hwnd: HWND) -> f32 {
    let dpi = GetDpiForWindow(hwnd);
    if dpi == 0 {
        1.0
    } else {
        dpi as f32 / 96.0
    }
}

/// Attempts to set per-monitor DPI awareness for the process.
/// Silently fails if already set (e.g., by the host application).
pub(crate) unsafe fn try_set_dpi_awareness() {
    let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
}
