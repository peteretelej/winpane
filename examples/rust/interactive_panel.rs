//! Demo: interactive floating panel with clickable buttons
//!
//! Creates a panel with interactive rect "buttons" that respond to clicks
//! and hover events. Demonstrates: Panel creation, interactive elements,
//! event polling, hover feedback, and drag handle.
//!
//! Run on Windows: cargo run -p winpane --example interactive_panel

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    use winpane::{Anchor, Color, Context, Event, PanelConfig, Placement, RectElement, TextElement};

    // ── CLI flags ──────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: interactive_panel [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --no-titlebar       Hide title bar, drag anywhere");
        println!("  --opacity <0.0-1.0> Surface opacity");
        println!("  --backdrop <type>   Backdrop: mica, acrylic");
        println!("  --capture-excluded  Hide from screenshots");
        println!("  --position <X,Y>    Explicit position");
        println!("  --monitor <N>       Monitor index (0=primary)");
        std::process::exit(0);
    }

    let no_titlebar = args.iter().any(|a| a == "--no-titlebar");
    let capture_excluded = args.iter().any(|a| a == "--capture-excluded");

    let opacity: f32 = args.iter().position(|a| a == "--opacity")
        .and_then(|i| args.get(i + 1)?.parse().ok())
        .unwrap_or(1.0);

    let backdrop = args.iter().position(|a| a == "--backdrop")
        .and_then(|i| args.get(i + 1).map(String::as_str))
        .and_then(|s| match s {
            "mica" => Some(winpane::Backdrop::Mica),
            "acrylic" => Some(winpane::Backdrop::Acrylic),
            _ => None,
        });

    let monitor_index: usize = args.iter().position(|a| a == "--monitor")
        .and_then(|i| args.get(i + 1)?.parse().ok())
        .unwrap_or(0);

    let explicit_position: Option<(i32, i32)> = args.iter().position(|a| a == "--position")
        .and_then(|i| {
            let val = args.get(i + 1)?;
            let parts: Vec<&str> = val.split(',').collect();
            Some((parts.first()?.parse().ok()?, parts.get(1)?.parse().ok()?))
        });
    // ───────────────────────────────────────────────────────────────

    let placement = if let Some((x, y)) = explicit_position {
        Placement::Position { x, y }
    } else {
        Placement::Monitor { index: monitor_index, anchor: Anchor::TopLeft, margin: 40 }
    };

    let ctx = Context::new()?;

    let panel = ctx.create_panel(PanelConfig {
        placement,
        width: 280,
        height: 220,
        draggable: true,
        drag_height: if no_titlebar { 220 } else { 32 },
        position_key: Some("interactive_panel".into()),
    })?;

    if let Some(bd) = backdrop { panel.set_backdrop(bd); }
    if capture_excluded { panel.set_capture_excluded(true); }
    if opacity < 1.0 { panel.set_opacity(opacity); }

    // Dark background
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 220.0,
            fill: Color::rgba(18, 18, 22, 255),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 23)),
            border_width: 1.0,
            interactive: false,
        },
    );

    // Title bar (drag area)
    if !no_titlebar {
        panel.set_text(
            "title",
            TextElement {
                text: "Interactive Panel".into(),
                x: 12.0,
                y: 8.0,
                font_size: 16.0,
                color: Color::rgba(232, 232, 237, 255),
                bold: true,
                ..Default::default()
            },
        );
    }

    // Separator
    panel.set_rect(
        "sep",
        RectElement {
            x: 12.0,
            y: 34.0,
            width: 256.0,
            height: 1.0,
            fill: Color::rgba(255, 255, 255, 18),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            interactive: false,
        },
    );

    // Button 1
    panel.set_rect(
        "btn_hello",
        RectElement {
            x: 20.0,
            y: 50.0,
            width: 240.0,
            height: 40.0,
            fill: Color::rgba(38, 38, 44, 255),
            corner_radius: 6.0,
            border_color: Some(Color::rgba(255, 255, 255, 23)),
            border_width: 1.0,
            interactive: true,
        },
    );
    panel.set_text(
        "btn_hello_text",
        TextElement {
            text: "Say Hello".into(),
            x: 100.0,
            y: 60.0,
            font_size: 13.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );

    // Button 2
    panel.set_rect(
        "btn_count",
        RectElement {
            x: 20.0,
            y: 100.0,
            width: 240.0,
            height: 40.0,
            fill: Color::rgba(38, 38, 44, 255),
            corner_radius: 6.0,
            border_color: Some(Color::rgba(255, 255, 255, 23)),
            border_width: 1.0,
            interactive: true,
        },
    );
    panel.set_text(
        "btn_count_text",
        TextElement {
            text: "Count: 0".into(),
            x: 100.0,
            y: 110.0,
            font_size: 13.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );

    // Status text
    panel.set_text(
        "status",
        TextElement {
            text: "Click a button or drag the title bar".into(),
            x: 20.0,
            y: 160.0,
            font_size: 11.0,
            color: Color::rgba(96, 96, 107, 255),
            ..Default::default()
        },
    );

    panel.show();

    println!("Interactive panel visible at (200, 200). Press Ctrl+C to exit.");

    let mut count = 0u32;
    let panel_id = panel.id();

    loop {
        while let Some(event) = ctx.poll_event() {
            match event {
                Event::ElementClicked {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => match key.as_str() {
                    "btn_hello" => {
                        println!("Hello from winpane!");
                        panel.set_text(
                            "status",
                            TextElement {
                                text: "Hello from winpane!".into(),
                                x: 20.0,
                                y: 160.0,
                                font_size: 11.0,
                                color: Color::rgba(52, 211, 153, 255),
                                ..Default::default()
                            },
                        );
                    }
                    "btn_count" => {
                        count += 1;
                        println!("Count: {count}");
                        panel.set_text(
                            "btn_count_text",
                            TextElement {
                                text: format!("Count: {count}"),
                                x: 100.0,
                                y: 110.0,
                                font_size: 13.0,
                                color: Color::rgba(232, 232, 237, 255),
                                ..Default::default()
                            },
                        );
                    }
                    _ => {}
                },
                Event::ElementHovered {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => {
                    if key == "btn_hello" || key == "btn_count" {
                        let y = if key == "btn_hello" { 50.0 } else { 100.0 };
                        panel.set_rect(
                            key,
                            RectElement {
                                x: 20.0,
                                y,
                                width: 240.0,
                                height: 40.0,
                                fill: Color::rgba(48, 48, 56, 255),
                                corner_radius: 6.0,
                                border_color: Some(Color::rgba(255, 255, 255, 31)),
                                border_width: 1.0,
                                interactive: true,
                            },
                        );
                    }
                }
                Event::ElementLeft {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => {
                    if key == "btn_hello" || key == "btn_count" {
                        let y = if key == "btn_hello" { 50.0 } else { 100.0 };
                        panel.set_rect(
                            key,
                            RectElement {
                                x: 20.0,
                                y,
                                width: 240.0,
                                height: 40.0,
                                fill: Color::rgba(38, 38, 44, 255),
                                corner_radius: 6.0,
                                border_color: Some(Color::rgba(255, 255, 255, 23)),
                                border_width: 1.0,
                                interactive: true,
                            },
                        );
                    }
                }
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
