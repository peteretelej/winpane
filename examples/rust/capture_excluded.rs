//! Demo: capture exclusion
//!
//! Creates a HUD overlay that is excluded from screen captures.
//! Take a screenshot while this is running to verify the overlay
//! is invisible (Win10 2004+) or shows as a black rectangle (older).
//!
//! Run on Windows: cargo run -p winpane --example capture_excluded

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

use winpane::{Color, Context, HudConfig, Placement, RectElement, TextElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        placement: Placement::Position { x: 100, y: 100 },
        width: 350,
        height: 150,
        position_key: None,
    })?;

    hud.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 350.0,
            height: 150.0,
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 31)),
            border_width: 1.0,
            interactive: false,
        },
    );

    hud.set_text(
        "label",
        TextElement {
            text: "CAPTURE EXCLUDED".into(),
            x: 20.0,
            y: 20.0,
            font_size: 16.0,
            color: Color::rgba(239, 68, 68, 255),
            bold: true,
            ..Default::default()
        },
    );

    hud.set_text(
        "info",
        TextElement {
            text: "This overlay is invisible in screenshots.\nTake a screenshot to verify.".into(),
            x: 20.0,
            y: 60.0,
            font_size: 13.0,
            color: Color::rgba(148, 148, 160, 255),
            ..Default::default()
        },
    );

    // Enable capture exclusion
    hud.set_capture_excluded(true);
    hud.show();

    println!("Capture-excluded HUD at (100, 100). Take a screenshot to test.");
    println!("Press Ctrl+C to exit.");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
