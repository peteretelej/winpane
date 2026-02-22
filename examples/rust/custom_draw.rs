//! Demo: custom draw escape hatch
//!
//! Creates a HUD and uses DrawOp to render custom content beyond
//! the retained-mode scene graph: filled rects, text, lines, ellipses.
//!
//! Run on Windows: cargo run -p winpane --example custom_draw

use winpane::{Color, Context, DrawOp, HudConfig, RectElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        x: 200,
        y: 200,
        width: 400,
        height: 300,
    })?;

    // Add a retained-mode background
    hud.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
            fill: Color::rgba(15, 15, 25, 220),
            corner_radius: 8.0,
            border_color: Some(Color::rgba(60, 60, 100, 180)),
            border_width: 1.0,
            interactive: false,
        },
    );

    hud.show();

    // Give the window time to appear
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Custom draw: bar chart with labels
    let bar_colors = [
        Color::rgba(80, 160, 255, 255),
        Color::rgba(100, 220, 160, 255),
        Color::rgba(255, 180, 80, 255),
        Color::rgba(255, 100, 120, 255),
    ];
    let bar_values: [f32; 4] = [0.7, 0.45, 0.9, 0.3];
    let bar_labels = ["Mon", "Tue", "Wed", "Thu"];

    let mut ops = Vec::new();

    // Title
    ops.push(DrawOp::DrawText {
        x: 20.0,
        y: 15.0,
        text: "Weekly Activity".into(),
        font_size: 18.0,
        color: Color::WHITE,
    });

    // Horizontal baseline
    ops.push(DrawOp::DrawLine {
        x1: 40.0,
        y1: 240.0,
        x2: 370.0,
        y2: 240.0,
        color: Color::rgba(80, 80, 120, 200),
        stroke_width: 1.0,
    });

    // Bars
    let bar_width = 60.0;
    let bar_max_height = 170.0;
    let start_x = 55.0;
    let spacing = 80.0;

    for (i, (&value, &color)) in bar_values.iter().zip(bar_colors.iter()).enumerate() {
        let x = start_x + i as f32 * spacing;
        let bar_height = value * bar_max_height;
        let y = 240.0 - bar_height;

        ops.push(DrawOp::FillRoundedRect {
            x,
            y,
            width: bar_width,
            height: bar_height,
            radius: 4.0,
            color,
        });

        // Label below bar
        ops.push(DrawOp::DrawText {
            x: x + 15.0,
            y: 248.0,
            text: bar_labels[i].into(),
            font_size: 12.0,
            color: Color::rgba(160, 160, 180, 255),
        });

        // Value above bar
        ops.push(DrawOp::DrawText {
            x: x + 12.0,
            y: y - 20.0,
            text: format!("{}%", (value * 100.0) as u32),
            font_size: 11.0,
            color,
        });
    }

    // Decorative ellipse
    ops.push(DrawOp::StrokeEllipse {
        cx: 370.0,
        cy: 30.0,
        rx: 12.0,
        ry: 12.0,
        color: Color::rgba(100, 180, 255, 120),
        stroke_width: 1.5,
    });

    hud.custom_draw(ops);

    println!("winpane custom_draw: overlay with bar chart at (200, 200).");
    println!("Press Ctrl+C to exit.");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
