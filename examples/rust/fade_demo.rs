//! Demo: fade-in and fade-out animations
//!
//! Creates a HUD with a dark background and text. Fades in over 500ms,
//! waits 3 seconds, updates the text, waits 1 second, then fades out
//! over 800ms using DirectComposition opacity animations.
//!
//! Run on Windows: cargo run -p winpane --example fade_demo

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

use winpane::{Anchor, Color, Context, HudConfig, Placement, RectElement, TextElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        placement: Placement::Monitor { index: 0, anchor: Anchor::TopLeft, margin: 20 },
        width: 400,
        height: 200,
        position_key: None,
    })?;

    // Dark semi-transparent background
    hud.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 200.0,
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            ..Default::default()
        },
    );

    hud.set_text(
        "msg",
        TextElement {
            text: "Fading in...".into(),
            x: 20.0,
            y: 80.0,
            font_size: 32.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );

    println!("winpane fade_demo: fading in...");
    hud.fade_in(500);
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Update text before fading out
    hud.set_text(
        "msg",
        TextElement {
            text: "Fading out...".into(),
            x: 20.0,
            y: 80.0,
            font_size: 32.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );

    println!("winpane fade_demo: fading out...");
    std::thread::sleep(std::time::Duration::from_secs(1));

    hud.fade_out(800);
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("winpane fade_demo: done.");
    Ok(())
}
