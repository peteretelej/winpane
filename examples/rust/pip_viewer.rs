//! Demo: PiP (Picture-in-Picture) thumbnail viewer
//!
//! Finds a window by title (default: "Notepad") and creates a live
//! DWM thumbnail preview of it.
//!
//! Run on Windows: cargo run -p winpane --example pip_viewer

use winpane::{Anchor, Context, Event, PipConfig, Placement};

#[cfg(target_os = "windows")]
fn find_window_by_title(title: &str) -> Option<isize> {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    use windows::core::HSTRING;
    let title = HSTRING::from(title);
    // SAFETY: FindWindowW is safe to call with a valid HSTRING; returns an error on failure.
    let hwnd = unsafe { FindWindowW(None, &title) }.ok()?;
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
    // ── CLI flags ──────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: pip_viewer [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --position <X,Y>    Explicit position");
        println!("  --monitor <N>       Monitor index (0=primary)");
        std::process::exit(0);
    }

    let monitor_index: usize = args
        .iter()
        .position(|a| a == "--monitor")
        .and_then(|i| args.get(i + 1)?.parse().ok())
        .unwrap_or(0);

    let explicit_position: Option<(i32, i32)> =
        args.iter().position(|a| a == "--position").and_then(|i| {
            let val = args.get(i + 1)?;
            let parts: Vec<&str> = val.split(',').collect();
            Some((parts.first()?.parse().ok()?, parts.get(1)?.parse().ok()?))
        });

    let placement = if let Some((x, y)) = explicit_position {
        Placement::Position { x, y }
    } else {
        Placement::Monitor {
            index: monitor_index,
            anchor: Anchor::BottomRight,
            margin: 20,
        }
    };
    // ───────────────────────────────────────────────────────────────

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
        placement,
        width: 400,
        height: 300,
        position_key: None,
    })?;

    pip.set_opacity(0.95);
    pip.show();

    println!("PiP viewer at (50, 50). Press Ctrl+C to exit.");

    loop {
        if let Some(Event::PipSourceClosed { .. }) = ctx.poll_event() {
            println!("Source window closed.");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(())
}
