use crate::types::{Anchor, MonitorInfo, Placement};

/// Enumerates connected display monitors, returning their geometry, DPI, and
/// primary status. Monitors are sorted: primary first, then by `x` ascending.
///
/// Falls back to a single 1920×1080 96-DPI monitor on failure or non-Windows.
#[cfg(target_os = "windows")]
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    use std::mem::size_of;
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::core::BOOL;

    // MONITORINFOF_PRIMARY = 1
    const MONITORINFOF_PRIMARY: u32 = 1;

    let mut results: Vec<MonitorInfo> = Vec::new();

    unsafe extern "system" fn callback(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        // SAFETY: lparam carries a valid pointer to our `results` Vec allocated on
        // the caller's stack; EnumDisplayMonitors guarantees sequential callbacks.
        let results = unsafe { &mut *(lparam.0 as *mut Vec<MonitorInfo>) };

        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;

        // SAFETY: hmonitor is valid within this callback; info is correctly sized.
        if unsafe {
            GetMonitorInfoW(
                hmonitor,
                std::ptr::from_mut(&mut info).cast(),
            )
        }
        .as_bool()
        {
            let rc = info.monitorInfo.rcMonitor;
            let is_primary =
                (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) == MONITORINFOF_PRIMARY;

            let mut dpi_x: u32 = 96;
            let mut dpi_y: u32 = 96;
            // SAFETY: hmonitor valid, out-pointers are valid stack references.
            let _ = unsafe {
                GetDpiForMonitor(
                    hmonitor,
                    MDT_EFFECTIVE_DPI,
                    std::ptr::from_mut(&mut dpi_x),
                    std::ptr::from_mut(&mut dpi_y),
                )
            };

            results.push(MonitorInfo {
                x: rc.left,
                y: rc.top,
                width: (rc.right - rc.left) as u32,
                height: (rc.bottom - rc.top) as u32,
                dpi: dpi_x,
                is_primary,
            });
        }

        BOOL(1)
    }

    // SAFETY: EnumDisplayMonitors iterates all monitors; callback writes to `results`.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(callback),
            LPARAM(std::ptr::from_mut(&mut results) as isize),
        );
    }

    // Sort: primary first, then by x ascending (left-to-right)
    results.sort_by(|a, b| {
        b.is_primary
            .cmp(&a.is_primary)
            .then_with(|| a.x.cmp(&b.x))
    });

    if results.is_empty() {
        results.push(fallback_monitor());
    }

    results
}

#[cfg(not(target_os = "windows"))]
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    vec![fallback_monitor()]
}

fn fallback_monitor() -> MonitorInfo {
    MonitorInfo {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        dpi: 96,
        is_primary: true,
    }
}

/// Resolves a [`Placement`] to absolute (x, y) pixel coordinates.
///
/// - `Placement::Position` returns the coordinates unchanged.
/// - `Placement::Monitor` computes the position from monitor geometry and anchor.
///
/// Does not clamp to monitor bounds.
pub fn resolve_placement(
    placement: &Placement,
    width: u32,
    height: u32,
    monitors: &[MonitorInfo],
) -> (i32, i32) {
    match placement {
        Placement::Position { x, y } => (*x, *y),
        Placement::Monitor {
            index,
            anchor,
            margin,
        } => {
            let Some(mon) = monitors.get(*index).or(monitors.first()) else {
                return (0, 0);
            };
            let margin = *margin as i32;
            match anchor {
                Anchor::TopLeft => (mon.x + margin, mon.y + margin),
                Anchor::TopRight => (
                    mon.x + mon.width as i32 - width as i32 - margin,
                    mon.y + margin,
                ),
                Anchor::BottomLeft => (
                    mon.x + margin,
                    mon.y + mon.height as i32 - height as i32 - margin,
                ),
                Anchor::BottomRight => (
                    mon.x + mon.width as i32 - width as i32 - margin,
                    mon.y + mon.height as i32 - height as i32 - margin,
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_position_passthrough() {
        let monitors = vec![fallback_monitor()];
        let (x, y) = resolve_placement(
            &Placement::Position { x: 42, y: 99 },
            100,
            50,
            &monitors,
        );
        assert_eq!((x, y), (42, 99));
    }

    #[test]
    fn resolve_monitor_top_left() {
        let monitors = vec![MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            dpi: 96,
            is_primary: true,
        }];
        let (x, y) = resolve_placement(
            &Placement::Monitor {
                index: 0,
                anchor: Anchor::TopLeft,
                margin: 20,
            },
            300,
            200,
            &monitors,
        );
        assert_eq!((x, y), (20, 20));
    }

    #[test]
    fn resolve_monitor_bottom_right() {
        let monitors = vec![MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            dpi: 96,
            is_primary: true,
        }];
        let (x, y) = resolve_placement(
            &Placement::Monitor {
                index: 0,
                anchor: Anchor::BottomRight,
                margin: 20,
            },
            300,
            200,
            &monitors,
        );
        assert_eq!((x, y), (1600, 860));
    }

    #[test]
    fn resolve_monitor_fallback_index() {
        let monitors = vec![fallback_monitor()];
        let (x, y) = resolve_placement(
            &Placement::Monitor {
                index: 99, // out of bounds
                anchor: Anchor::TopLeft,
                margin: 10,
            },
            100,
            100,
            &monitors,
        );
        // Falls back to first monitor
        assert_eq!((x, y), (10, 10));
    }

    #[test]
    fn resolve_empty_monitors() {
        let (x, y) = resolve_placement(
            &Placement::Monitor {
                index: 0,
                anchor: Anchor::TopLeft,
                margin: 0,
            },
            100,
            100,
            &[],
        );
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn enumerate_returns_nonempty() {
        let monitors = enumerate_monitors();
        assert!(!monitors.is_empty());
        // First should be primary (or fallback)
        assert!(monitors[0].is_primary);
    }
}
