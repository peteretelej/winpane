//! Demo: capture exclusion
//!
//! Creates a HUD overlay that is excluded from screen captures.
//! Take a screenshot while this is running to verify the overlay
//! is invisible (Win10 2004+) or shows as a black rectangle (older).
//!
//! Run on Windows: cargo run -p winpane --example capture_excluded

use winpane::{Color, Context, HudConfig, RectElement, TextElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        x: 100,
        y: 100,
        width: 350,
        height: 150,
    })?;

    hud.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 350.0,
            height: 150.0,
            fill: Color::rgba(30, 10, 10, 230),
            corner_radius: 8.0,
            border_color: Some(Color::rgba(255, 80, 80, 200)),
            border_width: 2.0,
            interactive: false,
        },
    );

    hud.set_text(
        "label",
        TextElement {
            text: "CAPTURE EXCLUDED".into(),
            x: 20.0,
            y: 20.0,
            font_size: 20.0,
            color: Color::rgba(255, 80, 80, 255),
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
            font_size: 12.0,
            color: Color::rgba(200, 180, 180, 255),
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
