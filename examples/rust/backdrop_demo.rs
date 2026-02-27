//! Demo: Mica and Acrylic backdrop effects
//!
//! Creates two panels side by side: one with Mica backdrop, one with Acrylic.
//! Each has a title label with white text over the transparent backdrop.
//! Stays visible for 10 seconds.
//!
//! Requires Windows 11 22H2+ (build 22621) for backdrop effects.
//! On older versions, the panels appear with no backdrop (transparent).
//!
//! Run on Windows: cargo run -p winpane --example backdrop_demo

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

use winpane::{Backdrop, Color, Context, PanelConfig, RectElement, TextElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    // Left panel: Mica backdrop
    let panel1 = ctx.create_panel(PanelConfig {
        x: 100,
        y: 100,
        width: 300,
        height: 200,
        ..Default::default()
    })?;
    panel1.set_backdrop(Backdrop::Mica);
    panel1.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 200.0,
            fill: Color::rgba(18, 18, 22, 102),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            ..Default::default()
        },
    );
    panel1.set_text(
        "title",
        TextElement {
            text: "Mica Backdrop".into(),
            x: 20.0,
            y: 20.0,
            font_size: 16.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );
    panel1.show();

    // Right panel: Acrylic backdrop
    let panel2 = ctx.create_panel(PanelConfig {
        x: 450,
        y: 100,
        width: 300,
        height: 200,
        ..Default::default()
    })?;
    panel2.set_backdrop(Backdrop::Acrylic);
    panel2.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 200.0,
            fill: Color::rgba(18, 18, 22, 102),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            ..Default::default()
        },
    );
    panel2.set_text(
        "title",
        TextElement {
            text: "Acrylic Backdrop".into(),
            x: 20.0,
            y: 20.0,
            font_size: 16.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );
    panel2.show();

    println!("winpane backdrop_demo: two panels visible at (100,100) and (450,100).");
    println!("Left = Mica, Right = Acrylic. Closing in 10 seconds.");

    std::thread::sleep(std::time::Duration::from_secs(10));
    Ok(())
}
