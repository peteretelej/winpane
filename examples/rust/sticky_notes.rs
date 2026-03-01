//! Demo: sticky notes with tray icon
//!
//! A floating note panel toggled from the system tray. Left-click the
//! tray icon to show/hide. Right-click for a context menu with Quit.
//! Demonstrates Tray + Panel composition — the "background app with UI" pattern.
//!
//! Run on Windows: cargo run -p winpane --example sticky_notes

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
    use winpane::{
        Anchor, Backdrop, Color, Context, Event, MenuItem, PanelConfig, Placement, RectElement,
        TextElement, TrayConfig,
    };

    // ── CLI flags ──────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: sticky_notes [OPTIONS]");
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

    let opacity: f32 = args
        .iter()
        .position(|a| a == "--opacity")
        .and_then(|i| args.get(i + 1)?.parse().ok())
        .unwrap_or(1.0);

    let backdrop_arg = args
        .iter()
        .position(|a| a == "--backdrop")
        .and_then(|i| args.get(i + 1).map(String::as_str))
        .and_then(|s| match s {
            "mica" => Some(Backdrop::Mica),
            "acrylic" => Some(Backdrop::Acrylic),
            _ => None,
        });

    let monitor_index: usize = args
        .iter()
        .position(|a| a == "--monitor")
        .and_then(|i| args.get(i + 1)?.parse().ok())
        .unwrap_or(0);

    let explicit_position: Option<(i32, i32)> =
        args.iter().position(|a| a == "--position").and_then(|i| {
            let val = args.get(i + 1)?;
            let parts: Vec<&str> = val.split(',').collect();
            Some((parts.first()?.parse().ok()?, parts.get(1)?.parse().ok()?))
        });
    // ───────────────────────────────────────────────────────────────

    let placement = if let Some((x, y)) = explicit_position {
        Placement::Position { x, y }
    } else {
        Placement::Monitor {
            index: monitor_index,
            anchor: Anchor::TopLeft,
            margin: 40,
        }
    };

    let ctx = Context::new()?;

    // ── Tray icon: 32×32 amber square ──────────────────────────
    let icon_size = 32u32;
    let mut icon_data = vec![0u8; (icon_size * icon_size * 4) as usize];
    for y in 0..icon_size {
        for x in 0..icon_size {
            let i = ((y * icon_size + x) * 4) as usize;
            icon_data[i] = 251; // R — warning amber
            icon_data[i + 1] = 191; // G
            icon_data[i + 2] = 36; // B
            icon_data[i + 3] = 255; // A
        }
    }

    let tray = ctx.create_tray(TrayConfig {
        icon_rgba: icon_data,
        icon_width: icon_size,
        icon_height: icon_size,
        tooltip: "Sticky Notes".into(),
    })?;

    tray.set_menu(vec![
        MenuItem {
            id: 1,
            label: "Show".into(),
            enabled: true,
        },
        MenuItem {
            id: 2,
            label: "Hide".into(),
            enabled: true,
        },
        MenuItem {
            id: 99,
            label: "Quit".into(),
            enabled: true,
        },
    ]);

    // ── Panel: 240×160, draggable title bar ────────────────────
    let panel = ctx.create_panel(PanelConfig {
        placement,
        width: 240,
        height: 160,
        draggable: true,
        drag_height: if no_titlebar { 160 } else { 28 },
        position_key: Some("sticky_notes".into()),
    })?;

    panel.set_backdrop(backdrop_arg.unwrap_or(Backdrop::Mica));
    if capture_excluded {
        panel.set_capture_excluded(true);
    }
    if opacity < 1.0 {
        panel.set_opacity(opacity);
    }

    // Background rect (glass fallback — visible on Win10, tints Mica on Win11)
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 240.0,
            height: 160.0,
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            interactive: false,
        },
    );

    // Title
    if !no_titlebar {
        panel.set_text(
            "title",
            TextElement {
                text: "Notes".into(),
                x: 12.0,
                y: 8.0,
                font_size: 16.0,
                color: Color::rgba(232, 232, 237, 255),
                bold: true,
                ..Default::default()
            },
        );
    }

    // Close button — transparent hit-target rect
    panel.set_rect(
        "close_btn",
        RectElement {
            x: 214.0,
            y: 4.0,
            width: 20.0,
            height: 20.0,
            fill: Color::rgba(0, 0, 0, 0),
            corner_radius: 4.0,
            border_color: None,
            border_width: 0.0,
            interactive: true,
        },
    );

    // Close button — × glyph
    panel.set_text(
        "close_x",
        TextElement {
            text: "×".into(),
            x: 219.0,
            y: 4.0,
            font_size: 14.0,
            color: Color::rgba(148, 148, 160, 255),
            ..Default::default()
        },
    );

    // Separator
    panel.set_rect(
        "sep",
        RectElement {
            x: 12.0,
            y: 28.0,
            width: 216.0,
            height: 1.0,
            fill: Color::rgba(255, 255, 255, 18),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            interactive: false,
        },
    );

    // Note lines
    let notes = [
        ("note_1", 38.0, "Remember to review PR"),
        ("note_2", 56.0, "Deploy staging at 3pm"),
        ("note_3", 74.0, "Call dentist"),
        ("note_4", 92.0, "Buy groceries"),
        ("note_5", 110.0, "Update dependencies"),
    ];

    for (key, y, text) in &notes {
        panel.set_text(
            key,
            TextElement {
                text: (*text).into(),
                x: 16.0,
                y: *y,
                font_size: 13.0,
                color: Color::rgba(232, 232, 237, 255),
                ..Default::default()
            },
        );
    }

    // Wire tray popup and show panel on launch
    tray.set_popup(&panel);
    panel.show();

    let panel_id = panel.id();

    println!("Sticky Notes: tray icon created. Left-click to toggle, right-click for menu.");

    loop {
        while let Some(event) = ctx.poll_event() {
            match event {
                Event::TrayClicked { button } => {
                    println!("Tray clicked: {button:?}");
                }
                Event::TrayMenuItemClicked { id } => match id {
                    1 => panel.show(),
                    2 => panel.hide(),
                    99 => return Ok(()),
                    _ => {}
                },
                Event::ElementClicked {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => {
                    if key == "close_btn" {
                        panel.hide();
                    }
                }
                Event::ElementHovered {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => {
                    if key == "close_btn" {
                        panel.set_rect(
                            "close_btn",
                            RectElement {
                                x: 214.0,
                                y: 4.0,
                                width: 20.0,
                                height: 20.0,
                                fill: Color::rgba(239, 68, 68, 80),
                                corner_radius: 4.0,
                                border_color: None,
                                border_width: 0.0,
                                interactive: true,
                            },
                        );
                    }
                }
                Event::ElementLeft {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => {
                    if key == "close_btn" {
                        panel.set_rect(
                            "close_btn",
                            RectElement {
                                x: 214.0,
                                y: 4.0,
                                width: 20.0,
                                height: 20.0,
                                fill: Color::rgba(0, 0, 0, 0),
                                corner_radius: 4.0,
                                border_color: None,
                                border_width: 0.0,
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
