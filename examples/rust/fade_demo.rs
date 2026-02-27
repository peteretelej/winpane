//! Demo: fade-in and fade-out animations
//!
//! Creates a HUD with a dark background and text. Fades in over 500ms,
//! waits 3 seconds, updates the text, waits 1 second, then fades out
//! over 800ms using DirectComposition opacity animations.
//!
//! Run on Windows: cargo run -p winpane --example fade_demo

use winpane::{Color, Context, HudConfig, RectElement, TextElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        x: 100,
        y: 100,
        width: 400,
        height: 200,
    })?;

    // Dark semi-transparent background
    hud.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 200.0,
            fill: Color::rgba(0, 0, 0, 180),
            corner_radius: 12.0,
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
            color: Color::WHITE,
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
            color: Color::WHITE,
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
