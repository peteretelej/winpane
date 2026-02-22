use std::cell::RefCell;

use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::System::LibraryLoader::*,
    Win32::UI::HiDpi::*,
    Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT},
    Win32::UI::WindowsAndMessaging::*,
};

use crate::input::PanelState;
use crate::types::{Error, Event};

// --- DPI change queue (thread-local, same thread as message loop) ---

pub(crate) struct DpiChangeEvent {
    pub hwnd: HWND,
    pub new_dpi: u32,
}

thread_local! {
    pub(crate) static PENDING_DPI_CHANGES: RefCell<Vec<DpiChangeEvent>> = RefCell::new(Vec::new());
}

// --- Tray notification queue (thread-local, same thread as message loop) ---

pub(crate) struct TrayNotification {
    pub event: u32, // WM_LBUTTONUP, WM_RBUTTONUP, etc. from lparam
}

thread_local! {
    pub(crate) static PENDING_TRAY_EVENTS: RefCell<Vec<TrayNotification>> = RefCell::new(Vec::new());
}

/// Tray icon callback message. WM_APP (0x8000) is used for command wake;
/// WM_APP + 1 is used for tray icon notifications.
pub(crate) const WM_TRAY_CALLBACK: u32 = 0x8001;

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

        // Panel window class (interactive)
        let panel_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(panel_wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("winpane_panel"),
            ..Default::default()
        };
        RegisterClassExW(&panel_class);
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

pub(crate) unsafe fn create_panel_window(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<HWND, Error> {
    CreateWindowExW(
        WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        w!("winpane_panel"),
        w!("winpane_panel"),
        WS_POPUP,
        x,
        y,
        width as i32,
        height as i32,
        None,
        None,
        Some(
            GetModuleHandleW(None)
                .map_err(|e| Error::WindowCreation(format!("GetModuleHandleW: {e}")))?
                .into(),
        ),
        None,
    )
    .map_err(|e| Error::WindowCreation(format!("CreateWindowExW panel: {e}")))
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
                // Do NOT call PostQuitMessage(0) here. Only control_wndproc
                // should call PostQuitMessage (during Shutdown). If surface
                // wndprocs call it, dropping any single HUD would terminate
                // the entire engine message loop.
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

extern "system" fn panel_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        // Get PanelState from GWLP_USERDATA.
        // Returns 0 during CreateWindowExW and after cleanup.
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);

        match msg {
            WM_NCHITTEST => {
                if state_ptr == 0 {
                    return LRESULT(-1); // HTTRANSPARENT during creation
                }
                let state = &*(state_ptr as *const PanelState);

                // Extract screen coordinates from lparam
                let screen_x = (lparam.0 & 0xFFFF) as i16 as i32;
                let screen_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                // Convert to client coordinates (physical pixels)
                let mut pt = POINT {
                    x: screen_x,
                    y: screen_y,
                };
                let _ = ScreenToClient(hwnd, &mut pt);
                let cx = pt.x as f32;
                let cy = pt.y as f32;

                // 1. Check interactive elements first (priority over drag)
                if state.hit_test_map.hit_test(cx, cy).is_some() {
                    return LRESULT(1); // HTCLIENT - receives mouse events
                }

                // 2. Check drag region
                if state.draggable && cy < state.drag_height {
                    return LRESULT(2); // HTCAPTION - enables OS-native drag
                }

                // 3. Click-through
                LRESULT(-1) // HTTRANSPARENT
            }

            WM_MOUSEACTIVATE => {
                // Prevent the panel from stealing focus when clicked,
                // and eat the mouse message so it doesn't propagate.
                // MA_NOACTIVATEANDEAT = 4
                LRESULT(4)
            }

            WM_LBUTTONUP => {
                if state_ptr == 0 {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }
                let state = &*(state_ptr as *const PanelState);

                let cx = (lparam.0 & 0xFFFF) as i16 as f32;
                let cy = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

                if let Some(key) = state.hit_test_map.hit_test(cx, cy) {
                    let _ = state.event_sender.send(Event::ElementClicked {
                        surface_id: state.surface_id,
                        key: key.to_string(),
                    });
                }
                LRESULT(0)
            }

            WM_MOUSEMOVE => {
                if state_ptr == 0 {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }
                let state = &mut *(state_ptr as *mut PanelState);

                let cx = (lparam.0 & 0xFFFF) as i16 as f32;
                let cy = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

                let new_key = state.hit_test_map.hit_test(cx, cy).map(|s| s.to_string());

                // Detect hover changes
                if new_key != state.hovered_key {
                    // Leave old element
                    if let Some(ref old_key) = state.hovered_key {
                        let _ = state.event_sender.send(Event::ElementLeft {
                            surface_id: state.surface_id,
                            key: old_key.clone(),
                        });
                    }
                    // Enter new element
                    if let Some(ref key) = new_key {
                        let _ = state.event_sender.send(Event::ElementHovered {
                            surface_id: state.surface_id,
                            key: key.clone(),
                        });
                    }
                    state.hovered_key = new_key;
                }

                // Ensure we get WM_MOUSELEAVE when cursor exits the window
                if !state.tracking_mouse {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    let _ = TrackMouseEvent(&mut tme);
                    state.tracking_mouse = true;
                }

                LRESULT(0)
            }

            WM_MOUSELEAVE => {
                if state_ptr == 0 {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }
                let state = &mut *(state_ptr as *mut PanelState);

                // Cursor left the window entirely
                if let Some(ref key) = state.hovered_key {
                    let _ = state.event_sender.send(Event::ElementLeft {
                        surface_id: state.surface_id,
                        key: key.clone(),
                    });
                }
                state.hovered_key = None;
                state.tracking_mouse = false;

                LRESULT(0)
            }

            WM_DPICHANGED => {
                // Same handling as hud_wndproc
                let new_dpi = (wparam.0 & 0xFFFF) as u32;
                let suggested_rect = &*(lparam.0 as *const RECT);

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
                // Do NOT call PostQuitMessage(0) here. Only control_wndproc
                // should call PostQuitMessage (during Shutdown).
                DefWindowProcW(hwnd, msg, wparam, lparam)
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
            WM_TRAY_CALLBACK => {
                let notification = lparam.0 as u32;
                PENDING_TRAY_EVENTS.with(|events| {
                    events.borrow_mut().push(TrayNotification {
                        event: notification,
                    });
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
