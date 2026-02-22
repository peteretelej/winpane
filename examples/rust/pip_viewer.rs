//! Demo: PiP (Picture-in-Picture) thumbnail viewer
//!
//! Finds a window by title (default: "Notepad") and creates a live
//! DWM thumbnail preview of it.
//!
//! Run on Windows: cargo run -p winpane --example pip_viewer

use winpane::{Context, Event, PipConfig};

#[cfg(target_os = "windows")]
fn find_window_by_title(title: &str) -> Option<isize> {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    let title = HSTRING::from(title);
    // SAFETY: FindWindowW is safe to call with a valid HSTRING; returns null on failure.
    let hwnd = unsafe { FindWindowW(None, &title) };
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd.0 as isize)
    }
}

#[cfg(not(target_os = "windows"))]
fn find_window_by_title(_title: &str) -> Option<isize> {
    None
}

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    // Try common window titles
    let targets = ["Untitled - Notepad", "Notepad", "Calculator"];
    let source_hwnd = targets
        .iter()
        .find_map(|t| find_window_by_title(t))
        .expect("No target window found. Open Notepad and try again.");

    println!("Found target window: 0x{source_hwnd:x}");

    let ctx = Context::new()?;
    let pip = ctx.create_pip(PipConfig {
        source_hwnd,
        x: 50,
        y: 50,
        width: 400,
        height: 300,
    })?;

    pip.set_opacity(0.95);
    pip.show();

    println!("PiP viewer at (50, 50). Press Ctrl+C to exit.");

    loop {
        if let Some(event) = ctx.poll_event() {
            match event {
                Event::PipSourceClosed { .. } => {
                    println!("Source window closed.");
                    break;
                }
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(())
}
