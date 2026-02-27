//! Demo: window-anchored companion panel
//!
//! Creates an interactive panel anchored to another window's top-right corner.
//! The panel tracks the target as it moves and hides when the target minimizes.
//!
//! Run on Windows: cargo run -p winpane --example anchored_companion

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

use winpane::{Anchor, Color, Context, Event, PanelConfig, RectElement, TextElement};

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
    let targets = ["Untitled - Notepad", "Notepad", "Calculator"];
    let target_hwnd = targets
        .iter()
        .find_map(|t| find_window_by_title(t))
        .expect("No target window found. Open Notepad and try again.");

    println!("Anchoring to window: 0x{target_hwnd:x}");

    let ctx = Context::new()?;

    let panel = ctx.create_panel(PanelConfig {
        x: 0,
        y: 0,
        width: 180,
        height: 120,
        draggable: false,
        drag_height: 0,
    })?;

    // Background
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 180.0,
            height: 120.0,
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            interactive: false,
        },
    );

    panel.set_text(
        "title",
        TextElement {
            text: "Companion".into(),
            x: 12.0,
            y: 10.0,
            font_size: 16.0,
            color: Color::rgba(130, 170, 255, 255),
            bold: true,
            ..Default::default()
        },
    );

    panel.set_text(
        "info",
        TextElement {
            text: "Tracking target window.\nMove it around!".into(),
            x: 12.0,
            y: 40.0,
            font_size: 13.0,
            color: Color::rgba(148, 148, 160, 255),
            ..Default::default()
        },
    );

    // Anchor to top-right of target, offset 8px right
    panel.anchor_to(target_hwnd, Anchor::TopRight, (8, 0));
    panel.show();

    println!("Companion anchored at top-right. Move the target window to see it follow.");
    println!("Minimize the target to see the companion hide. Press Ctrl+C to exit.");

    loop {
        if let Some(Event::AnchorTargetClosed { .. }) = ctx.poll_event() {
            println!("Target window closed.");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(())
}
