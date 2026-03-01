//! Demo: system tray icon with popup panel
//!
//! Creates a tray icon with a colored square. Left-click toggles a popup
//! panel. Right-click shows a context menu with Show/Hide/Exit options.
//!
//! Run on Windows: cargo run -p winpane --example tray_ticker

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
        Color, Context, Event, MenuItem, PanelConfig, Placement, RectElement, TextElement,
        TrayConfig,
    };

    let ctx = Context::new()?;

    // Generate a simple 32x32 colored square icon (RGBA)
    let icon_size = 32u32;
    let mut icon_data = vec![0u8; (icon_size * icon_size * 4) as usize];
    for y in 0..icon_size {
        for x in 0..icon_size {
            let i = ((y * icon_size + x) * 4) as usize;
            icon_data[i] = 60; // R
            icon_data[i + 1] = 120; // G
            icon_data[i + 2] = 220; // B
            icon_data[i + 3] = 255; // A
        }
    }

    let tray = ctx.create_tray(TrayConfig {
        icon_rgba: icon_data,
        icon_width: icon_size,
        icon_height: icon_size,
        tooltip: "winpane tray demo".into(),
    })?;

    // Create popup panel
    let popup = ctx.create_panel(PanelConfig {
        placement: Placement::Position { x: 0, y: 0 },
        width: 220,
        height: 140,
        draggable: false,
        drag_height: 0,
        position_key: None,
    })?;

    popup.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 220.0,
            height: 140.0,
            fill: Color::rgba(18, 18, 22, 255),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 23)),
            border_width: 1.0,
            interactive: false,
        },
    );

    popup.set_text(
        "title",
        TextElement {
            text: "Tray Popup".into(),
            x: 16.0,
            y: 12.0,
            font_size: 16.0,
            color: Color::rgba(232, 232, 237, 255),
            bold: true,
            ..Default::default()
        },
    );

    popup.set_text(
        "info",
        TextElement {
            text: "Left-click tray to toggle\nRight-click for menu".into(),
            x: 16.0,
            y: 45.0,
            font_size: 13.0,
            color: Color::rgba(148, 148, 160, 255),
            ..Default::default()
        },
    );

    popup.set_text(
        "status",
        TextElement {
            text: "Status: idle".into(),
            x: 16.0,
            y: 100.0,
            font_size: 11.0,
            color: Color::rgba(52, 211, 153, 255),
            ..Default::default()
        },
    );

    // Associate popup with tray
    tray.set_popup(&popup);

    // Set right-click menu
    tray.set_menu(vec![
        MenuItem {
            id: 1,
            label: "Show Popup".into(),
            enabled: true,
        },
        MenuItem {
            id: 2,
            label: "Hide Popup".into(),
            enabled: true,
        },
        MenuItem {
            id: 99,
            label: "Exit".into(),
            enabled: true,
        },
    ]);

    println!("Tray icon created. Look in the system tray.");
    println!("Left-click: toggle popup. Right-click: context menu.");

    let mut tick = 0u64;

    loop {
        while let Some(event) = ctx.poll_event() {
            match event {
                Event::TrayClicked { button } => {
                    println!("Tray clicked: {button:?}");
                }
                Event::TrayMenuItemClicked { id } => {
                    println!("Menu item: {id}");
                    match id {
                        1 => popup.show(),
                        2 => popup.hide(),
                        99 => {
                            println!("Exiting.");
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Update status periodically
        tick += 1;
        if tick.is_multiple_of(60) {
            let secs = tick / 60;
            popup.set_text(
                "status",
                TextElement {
                    text: format!("Status: uptime {secs}s"),
                    x: 16.0,
                    y: 100.0,
                    font_size: 11.0,
                    color: Color::rgba(52, 211, 153, 255),
                    ..Default::default()
                },
            );
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
