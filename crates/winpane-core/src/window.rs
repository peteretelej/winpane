use std::cell::RefCell;
use std::mem::size_of;

use windows::{
    Win32::Foundation::*,
    Win32::Graphics::Gdi::ScreenToClient,
    Win32::System::LibraryLoader::*,
    Win32::UI::Controls::WM_MOUSELEAVE,
    Win32::UI::HiDpi::*,
    Win32::UI::Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent},
    Win32::UI::WindowsAndMessaging::*,
    core::*,
};

use crate::input::PanelState;
use crate::types::{Error, Event};

// --- DPI change queue (thread-local, same thread as message loop) ---

pub(crate) struct DpiChangeEvent {
    pub hwnd: HWND,
    pub new_dpi: u32,
}

thread_local! {
    pub(crate) static PENDING_DPI_CHANGES: RefCell<Vec<DpiChangeEvent>> = const { RefCell::new(Vec::new()) };
}

// --- Tray notification queue (thread-local, same thread as message loop) ---

pub(crate) struct TrayNotification {
    pub event: u32, // WM_LBUTTONUP, WM_RBUTTONUP, etc. from lparam
}

thread_local! {
    pub(crate) static PENDING_TRAY_EVENTS: RefCell<Vec<TrayNotification>> = const { RefCell::new(Vec::new()) };
}

// --- Fade completion queue (thread-local, same thread as message loop) ---

pub(crate) struct FadeCompleteEvent {
    pub hwnd: HWND,
}

thread_local! {
    pub(crate) static PENDING_FADE_COMPLETIONS: RefCell<Vec<FadeCompleteEvent>> = const { RefCell::new(Vec::new()) };
}

// --- Position change queue (thread-local, same thread as message loop) ---

pub(crate) struct PositionChangeEvent {
    pub hwnd: isize,
    pub x: i32,
    pub y: i32,
}

thread_local! {
    pub(crate) static PENDING_POSITION_CHANGES: RefCell<Vec<PositionChangeEvent>> =
        const { RefCell::new(Vec::new()) };
}

/// Tray icon callback message. WM_APP (0x8000) is used for command wake;
/// WM_APP + 1 is used for tray icon notifications.
pub(crate) const WM_TRAY_CALLBACK: u32 = 0x8001;

// --- SendHwnd wrapper ---

/// Wrapper around HWND that implements Send.
#[derive(Clone, Copy)]
pub struct SendHwnd(pub HWND);
// SAFETY: PostMessage to an HWND is thread-safe by Win32 specification.
unsafe impl Send for SendHwnd {}

// --- Window class registration ---

static REGISTER_CLASSES: std::sync::Once = std::sync::Once::new();

pub(crate) unsafe fn ensure_classes_registered() {
    REGISTER_CLASSES.call_once(|| {
        // SAFETY: Window class registration with valid module handle and wndproc.
        unsafe {
            let instance = GetModuleHandleW(None).unwrap();

            let hud_class = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(hud_wndproc),
                hInstance: instance.into(),
                lpszClassName: w!("winpane_hud"),
                ..Default::default()
            };
            RegisterClassExW(&hud_class);

            let ctrl_class = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(control_wndproc),
                hInstance: instance.into(),
                lpszClassName: w!("winpane_control"),
                ..Default::default()
            };
            RegisterClassExW(&ctrl_class);

            // Panel window class (interactive)
            let panel_class = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(panel_wndproc),
                hInstance: instance.into(),
                lpszClassName: w!("winpane_panel"),
                ..Default::default()
            };
            RegisterClassExW(&panel_class);
        }
    });
}

// --- Window creation ---

pub(crate) unsafe fn create_hud_window(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> std::result::Result<HWND, Error> {
    // SAFETY: CreateWindowExW with registered class and valid parameters.
    unsafe {
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
}

pub(crate) unsafe fn create_panel_window(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> std::result::Result<HWND, Error> {
    // SAFETY: CreateWindowExW with registered class and valid parameters.
    unsafe {
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
}

pub(crate) unsafe fn create_control_window() -> std::result::Result<HWND, Error> {
    // SAFETY: CreateWindowExW with registered class and valid parameters.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("winpane_control"),
            w!("winpane_ctrl"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(GetModuleHandleW(None).unwrap().into()),
            None,
        )
        .map_err(|e| Error::WindowCreation(e.to_string()))
    }
}

// --- Window procedures ---

extern "system" fn hud_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: Win32 message handler with HWND/LPARAM from the OS.
    unsafe {
        match msg {
            WM_NCHITTEST => LRESULT(-1), // HTTRANSPARENT - click through

            WM_TIMER => {
                let timer_id = wparam.0;
                let _ = KillTimer(Some(hwnd), timer_id);
                PENDING_FADE_COMPLETIONS.with(|completions| {
                    completions.borrow_mut().push(FadeCompleteEvent { hwnd });
                });
                LRESULT(0)
            }

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

            WM_WINDOWPOSCHANGED => {
                let wp = &*(lparam.0 as *const WINDOWPOS);
                if (wp.flags.0 & SWP_NOMOVE.0) == 0 {
                    PENDING_POSITION_CHANGES.with(|q| {
                        q.borrow_mut().push(PositionChangeEvent {
                            hwnd: hwnd.0 as isize,
                            x: wp.x,
                            y: wp.y,
                        });
                    });
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

extern "system" fn panel_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: Win32 message handler with HWND/LPARAM from the OS.
    unsafe {
        // Get PanelState from GWLP_USERDATA.
        // Returns 0 during CreateWindowExW and after cleanup.
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);

        match msg {
            // Prevent panel from stealing focus when clicked
            WM_MOUSEACTIVATE => LRESULT(3), // MA_NOACTIVATE

            WM_SETCURSOR => {
                // LOWORD(lParam) is the hit-test code from WM_NCHITTEST.
                // HTCAPTION (2) means the cursor is over the drag region.
                if (lparam.0 & 0xFFFF) as u16 == 2 {
                    SetCursor(Some(LoadCursorW(None, IDC_SIZEALL).unwrap()));
                    return LRESULT(1); // TRUE — we handled the cursor
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

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
                let state = &*(state_ptr as *const PanelState);

                let cx = (lparam.0 & 0xFFFF) as i16 as f32;
                let cy = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

                let new_key = state.hit_test_map.hit_test(cx, cy).map(ToString::to_string);

                // Detect hover changes
                let current_key = state.hovered_key.borrow().clone();
                if new_key != current_key {
                    // Leave old element
                    if let Some(ref old_key) = current_key {
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
                    *state.hovered_key.borrow_mut() = new_key;
                }

                // Ensure we get WM_MOUSELEAVE when cursor exits the window
                if !state.tracking_mouse.get() {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    let _ = TrackMouseEvent(&mut tme);
                    state.tracking_mouse.set(true);
                }

                LRESULT(0)
            }

            WM_MOUSELEAVE => {
                if state_ptr == 0 {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }
                let state = &*(state_ptr as *const PanelState);

                // Cursor left the window entirely
                if let Some(key) = state.hovered_key.borrow().as_ref() {
                    let _ = state.event_sender.send(Event::ElementLeft {
                        surface_id: state.surface_id,
                        key: key.clone(),
                    });
                }
                *state.hovered_key.borrow_mut() = None;
                state.tracking_mouse.set(false);

                LRESULT(0)
            }

            WM_TIMER => {
                let timer_id = wparam.0;
                let _ = KillTimer(Some(hwnd), timer_id);
                PENDING_FADE_COMPLETIONS.with(|completions| {
                    completions.borrow_mut().push(FadeCompleteEvent { hwnd });
                });
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

            WM_WINDOWPOSCHANGED => {
                let wp = &*(lparam.0 as *const WINDOWPOS);
                if (wp.flags.0 & SWP_NOMOVE.0) == 0 {
                    PENDING_POSITION_CHANGES.with(|q| {
                        q.borrow_mut().push(PositionChangeEvent {
                            hwnd: hwnd.0 as isize,
                            x: wp.x,
                            y: wp.y,
                        });
                    });
                }
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
    // SAFETY: Win32 message handler with HWND/LPARAM from the OS.
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
    // SAFETY: GetDpiForWindow with valid HWND from caller.
    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 }
    }
}

/// Attempts to set per-monitor DPI awareness for the process.
/// Silently fails if already set (e.g., by the host application).
pub(crate) unsafe fn try_set_dpi_awareness() {
    // SAFETY: DPI awareness setting; harmless if already set.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

// --- Capture exclusion utilities ---

#[cfg(target_os = "windows")]
static WINDOWS_BUILD_NUMBER: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

#[cfg(target_os = "windows")]
pub(crate) fn get_windows_build_number() -> u32 {
    *WINDOWS_BUILD_NUMBER.get_or_init(||
        // SAFETY: RtlGetVersion via GetProcAddress from ntdll.dll.
        unsafe { rtl_get_version_build() })
}

#[cfg(target_os = "windows")]
unsafe fn rtl_get_version_build() -> u32 {
    use windows::Win32::Foundation::NTSTATUS;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;

    type RtlGetVersionFn = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> NTSTATUS;

    // SAFETY: RtlGetVersion via GetProcAddress from ntdll.dll.
    unsafe {
        let Ok(ntdll) = GetModuleHandleW(w!("ntdll.dll")) else {
            return 0;
        };

        let Some(proc) = GetProcAddress(ntdll, windows::core::s!("RtlGetVersion")) else {
            return 0;
        };

        let rtl_get_version: RtlGetVersionFn = std::mem::transmute(proc);
        let mut info: OSVERSIONINFOW = std::mem::zeroed();
        info.dwOSVersionInfoSize = size_of::<OSVERSIONINFOW>() as u32;

        let status = rtl_get_version(&mut info);
        if status.is_ok() {
            info.dwBuildNumber
        } else {
            0
        }
    }
}

/// Returns true if the current Windows build supports WDA_EXCLUDEFROMCAPTURE.
/// Win10 2004 = build 19041.
#[cfg(target_os = "windows")]
pub(crate) fn supports_exclude_from_capture() -> bool {
    get_windows_build_number() >= 19041
}

#[cfg(target_os = "windows")]
pub(crate) unsafe fn set_capture_excluded(hwnd: HWND, excluded: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WINDOW_DISPLAY_AFFINITY,
    };

    let affinity = if !excluded {
        WINDOW_DISPLAY_AFFINITY(0) // WDA_NONE
    } else if supports_exclude_from_capture() {
        WINDOW_DISPLAY_AFFINITY(0x00000011) // WDA_EXCLUDEFROMCAPTURE
    } else {
        WINDOW_DISPLAY_AFFINITY(0x00000001) // WDA_MONITOR
    };

    // SAFETY: SetWindowDisplayAffinity with valid HWND.
    unsafe {
        let _ = SetWindowDisplayAffinity(hwnd, affinity);
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) unsafe fn set_capture_excluded(_hwnd: HWND, _excluded: bool) {}

// --- Backdrop (DWM system backdrop) utilities ---

/// Returns true if the current Windows build supports DWMWA_SYSTEMBACKDROP_TYPE.
/// Win11 22H2 = build 22621.
#[cfg(target_os = "windows")]
pub(crate) fn supports_backdrop() -> bool {
    get_windows_build_number() >= 22621
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn supports_backdrop() -> bool {
    false
}

/// Sets the DWM system backdrop type on a window. Returns false on unsupported Windows.
/// Requires extending the DWM frame into the client area first.
#[cfg(target_os = "windows")]
pub(crate) unsafe fn set_window_backdrop(hwnd: HWND, backdrop: crate::types::Backdrop) -> bool {
    use windows::Win32::Graphics::Dwm::{DwmExtendFrameIntoClientArea, DwmSetWindowAttribute};
    use windows::Win32::UI::Controls::MARGINS;

    if !supports_backdrop() {
        return false;
    }

    // SAFETY: DWM APIs with valid HWND and correct attribute size.
    unsafe {
        // Extend DWM frame over entire client area (required for backdrop to show)
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

        // DWMWA_SYSTEMBACKDROP_TYPE = 38
        let backdrop_type: u32 = match backdrop {
            crate::types::Backdrop::None => 1,    // DWMSBT_NONE
            crate::types::Backdrop::Mica => 2,    // DWMSBT_MAINWINDOW
            crate::types::Backdrop::Acrylic => 3, // DWMSBT_TRANSIENTWINDOW
        };

        DwmSetWindowAttribute(
            hwnd,
            windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(38),
            std::ptr::from_ref(&backdrop_type).cast::<std::ffi::c_void>(),
            size_of::<u32>() as u32,
        )
        .is_ok()
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) unsafe fn set_window_backdrop(_hwnd: HWND, _backdrop: crate::types::Backdrop) -> bool {
    false
}
