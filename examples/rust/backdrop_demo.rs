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

use winpane::{Backdrop, Color, Context, PanelConfig, TextElement};

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
    panel1.set_text(
        "title",
        TextElement {
            text: "Mica Backdrop".into(),
            x: 20.0,
            y: 20.0,
            font_size: 24.0,
            color: Color::WHITE,
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
    panel2.set_text(
        "title",
        TextElement {
            text: "Acrylic Backdrop".into(),
            x: 20.0,
            y: 20.0,
            font_size: 24.0,
            color: Color::WHITE,
            ..Default::default()
        },
    );
    panel2.show();

    println!("winpane backdrop_demo: two panels visible at (100,100) and (450,100).");
    println!("Left = Mica, Right = Acrylic. Closing in 10 seconds.");

    std::thread::sleep(std::time::Duration::from_secs(10));
    Ok(())
}
