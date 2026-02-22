//! Demo: floating stats HUD overlay
//!
//! Creates a semi-transparent panel with live-updating stats text.
//! Demonstrates: Context creation, HUD surface, text/rect elements,
//! live updates, and element layering.
//!
//! Run on Windows: cargo run -p winpane --example hud_demo

use winpane::{Color, Context, HudConfig, RectElement, TextElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        x: 100,
        y: 100,
        width: 320,
        height: 180,
    })?;

    // Dark semi-transparent background with rounded corners
    hud.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 180.0,
            fill: Color::rgba(20, 20, 30, 200),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(80, 80, 120, 150)),
            border_width: 1.0,
        },
    );

    // Title
    hud.set_text(
        "title",
        TextElement {
            text: "System Monitor".into(),
            x: 16.0,
            y: 12.0,
            font_size: 18.0,
            color: Color::WHITE,
            bold: true,
            ..Default::default()
        },
    );

    // Separator line (thin rect)
    hud.set_rect(
        "sep",
        RectElement {
            x: 16.0,
            y: 40.0,
            width: 288.0,
            height: 1.0,
            fill: Color::rgba(100, 100, 140, 100),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
        },
    );

    // Stat labels (muted color)
    let label_color = Color::rgba(160, 160, 180, 255);

    hud.set_text(
        "cpu_label",
        TextElement {
            text: "CPU".into(),
            x: 16.0,
            y: 52.0,
            font_size: 13.0,
            color: label_color,
            ..Default::default()
        },
    );

    hud.set_text(
        "mem_label",
        TextElement {
            text: "Memory".into(),
            x: 16.0,
            y: 80.0,
            font_size: 13.0,
            color: label_color,
            ..Default::default()
        },
    );

    hud.set_text(
        "uptime_label",
        TextElement {
            text: "Uptime".into(),
            x: 16.0,
            y: 108.0,
            font_size: 13.0,
            color: label_color,
            ..Default::default()
        },
    );

    hud.show();

    println!("winpane hud_demo: overlay should be visible at (100, 100).");
    println!("Stats update every second. Press Ctrl+C to exit.");

    // Simulate live updates
    let mut tick = 0u64;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        tick += 1;

        // Fake fluctuating CPU value
        let cpu = 30 + ((tick * 7) % 50) as u8;
        hud.set_text(
            "cpu_val",
            TextElement {
                text: format!("{cpu}%"),
                x: 200.0,
                y: 52.0,
                font_size: 13.0,
                color: if cpu > 70 {
                    Color::rgba(255, 100, 100, 255)
                } else {
                    Color::rgba(100, 220, 160, 255)
                },
                ..Default::default()
            },
        );

        // Fake memory value
        let mem_gb = 8.0 + (tick % 4) as f32 * 0.3;
        hud.set_text(
            "mem_val",
            TextElement {
                text: format!("{mem_gb:.1} GB / 16 GB"),
                x: 200.0,
                y: 80.0,
                font_size: 13.0,
                color: Color::rgba(100, 180, 255, 255),
                ..Default::default()
            },
        );

        // Uptime counter
        let hrs = tick / 3600;
        let mins = (tick % 3600) / 60;
        let secs = tick % 60;
        hud.set_text(
            "uptime_val",
            TextElement {
                text: format!("{hrs:02}:{mins:02}:{secs:02}"),
                x: 200.0,
                y: 108.0,
                font_size: 13.0,
                color: Color::rgba(200, 200, 220, 255),
                ..Default::default()
            },
        );
    }
}
