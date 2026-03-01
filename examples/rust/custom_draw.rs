//! Demo: custom draw escape hatch
//!
//! Creates a HUD and uses DrawOp to render custom content beyond
//! the retained-mode scene graph: filled rects, text, lines, ellipses.
//!
//! Run on Windows: cargo run -p winpane --example custom_draw

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

use winpane::{Color, Context, DrawOp, HudConfig, Placement, RectElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        placement: Placement::Position { x: 200, y: 200 },
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
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            interactive: false,
        },
    );

    hud.show();

    // Give the window time to appear
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Custom draw: bar chart with labels
    let bar_colors = [
        Color::rgba(82, 139, 255, 255),
        Color::rgba(52, 211, 153, 255),
        Color::rgba(251, 191, 36, 255),
        Color::rgba(239, 68, 68, 255),
    ];
    let bar_values: [f32; 4] = [0.7, 0.45, 0.9, 0.3];
    let bar_labels = ["Mon", "Tue", "Wed", "Thu"];

    let mut ops = Vec::new();

    // Title
    ops.push(DrawOp::DrawText {
        x: 20.0,
        y: 15.0,
        text: "Weekly Activity".into(),
        font_size: 16.0,
        color: Color::rgba(232, 232, 237, 255),
    });

    // Horizontal baseline
    ops.push(DrawOp::DrawLine {
        x1: 40.0,
        y1: 240.0,
        x2: 370.0,
        y2: 240.0,
        color: Color::rgba(255, 255, 255, 18),
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
            font_size: 13.0,
            color: Color::rgba(148, 148, 160, 255),
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
        color: Color::rgba(82, 139, 255, 120),
        stroke_width: 1.5,
    });

    hud.custom_draw(ops);

    println!("winpane custom_draw: overlay with bar chart at (200, 200).");
    println!("Press Ctrl+C to exit.");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
