//! Demo: interactive floating panel with clickable buttons
//!
//! Creates a panel with interactive rect "buttons" that respond to clicks
//! and hover events. Demonstrates: Panel creation, interactive elements,
//! event polling, hover feedback, and drag handle.
//!
//! Run on Windows: cargo run -p winpane --example interactive_panel

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    use winpane::{Color, Context, Event, PanelConfig, RectElement, TextElement};

    let ctx = Context::new()?;

    let panel = ctx.create_panel(PanelConfig {
        x: 200,
        y: 200,
        width: 280,
        height: 220,
        draggable: true,
        drag_height: 36,
    })?;

    // Dark background
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 220.0,
            fill: Color::rgba(25, 25, 35, 230),
            corner_radius: 8.0,
            border_color: Some(Color::rgba(60, 60, 90, 200)),
            border_width: 1.0,
            interactive: false,
        },
    );

    // Title bar (drag area)
    panel.set_text(
        "title",
        TextElement {
            text: "Interactive Panel".into(),
            x: 12.0,
            y: 8.0,
            font_size: 15.0,
            color: Color::WHITE,
            bold: true,
            ..Default::default()
        },
    );

    // Separator
    panel.set_rect(
        "sep",
        RectElement {
            x: 12.0,
            y: 36.0,
            width: 256.0,
            height: 1.0,
            fill: Color::rgba(80, 80, 120, 120),
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
            fill: Color::rgba(50, 80, 140, 200),
            corner_radius: 6.0,
            border_color: Some(Color::rgba(80, 120, 200, 180)),
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
            font_size: 14.0,
            color: Color::WHITE,
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
            fill: Color::rgba(50, 80, 140, 200),
            corner_radius: 6.0,
            border_color: Some(Color::rgba(80, 120, 200, 180)),
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
            font_size: 14.0,
            color: Color::WHITE,
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
            color: Color::rgba(140, 140, 180, 200),
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
                                color: Color::rgba(100, 220, 160, 255),
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
                                font_size: 14.0,
                                color: Color::WHITE,
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
                                fill: Color::rgba(70, 100, 170, 220),
                                corner_radius: 6.0,
                                border_color: Some(Color::rgba(100, 140, 220, 220)),
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
                                fill: Color::rgba(50, 80, 140, 200),
                                corner_radius: 6.0,
                                border_color: Some(Color::rgba(80, 120, 200, 180)),
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
