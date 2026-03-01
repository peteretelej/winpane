//! Demo: interactive countdown timer panel
//!
//! Creates a 5-minute countdown panel with Start/Pause and Reset buttons.
//! The timer text color transitions from green to yellow to pulsing red as
//! time runs out. Demonstrates: Panel creation, interactive elements, event
//! polling, hover feedback, drag handle, time-based visual changes.
//!
//! Run on Windows: cargo run -p winpane --example countdown_timer

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

const INITIAL_SECS: u32 = 300; // 5 minutes

#[derive(PartialEq, Clone, Copy)]
enum TimerState {
    Idle,
    Running,
    Paused,
}

fn format_time(total_secs: u32) -> String {
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}")
}

fn timer_color(remaining: u32, state: &TimerState, elapsed_ms: u128) -> winpane::Color {
    use winpane::Color;
    match state {
        TimerState::Idle => Color::rgba(232, 232, 237, 255),
        TimerState::Paused => Color::rgba(148, 148, 160, 255),
        TimerState::Running if remaining > 30 => Color::rgba(52, 211, 153, 255),
        TimerState::Running if remaining > 10 => Color::rgba(251, 191, 36, 255),
        TimerState::Running => {
            // Pulsing red: sin wave alpha 128–255 at ~1Hz
            let pulse = ((elapsed_ms % 1000) as f64 / 1000.0 * std::f64::consts::TAU).sin();
            let alpha = (191.5 + 63.5 * pulse) as u8;
            Color::rgba(239, 68, 68, alpha)
        }
    }
}

fn normal_rect(x: f32, y: f32, width: f32) -> winpane::RectElement {
    use winpane::{Color, RectElement};
    RectElement {
        x,
        y,
        width,
        height: 34.0,
        fill: Color::rgba(38, 38, 44, 255),
        corner_radius: 6.0,
        border_color: Some(Color::rgba(255, 255, 255, 23)),
        border_width: 1.0,
        interactive: true,
    }
}

fn hover_rect(x: f32, y: f32, width: f32) -> winpane::RectElement {
    use winpane::{Color, RectElement};
    RectElement {
        x,
        y,
        width,
        height: 34.0,
        fill: Color::rgba(48, 48, 56, 255),
        corner_radius: 6.0,
        border_color: Some(Color::rgba(255, 255, 255, 31)),
        border_width: 1.0,
        interactive: true,
    }
}

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    use std::thread;
    use std::time::{Duration, Instant};
    use winpane::{
        Anchor, Color, Context, Event, PanelConfig, Placement, RectElement, TextElement,
    };

    // ── CLI flags ──────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: countdown_timer [OPTIONS]");
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
    let tb_h = if no_titlebar { 0u32 } else { 28 };
    let capture_excluded = args.iter().any(|a| a == "--capture-excluded");

    let opacity: f32 = args
        .iter()
        .position(|a| a == "--opacity")
        .and_then(|i| args.get(i + 1)?.parse().ok())
        .unwrap_or(1.0);

    let backdrop = args
        .iter()
        .position(|a| a == "--backdrop")
        .and_then(|i| args.get(i + 1).map(String::as_str))
        .and_then(|s| match s {
            "mica" => Some(winpane::Backdrop::Mica),
            "acrylic" => Some(winpane::Backdrop::Acrylic),
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

    // Layout positions (adjusted for optional title bar)
    let sep_y = tb_h as f32;
    let timer_y = tb_h as f32 + 10.0;
    let btn_y = tb_h as f32 + 64.0;
    let btn_text_y = tb_h as f32 + 72.0;

    let placement = if let Some((x, y)) = explicit_position {
        Placement::Position { x, y }
    } else {
        Placement::Monitor {
            index: monitor_index,
            anchor: Anchor::BottomRight,
            margin: 130,
        }
    };

    let ctx = Context::new()?;

    let panel = ctx.create_panel(PanelConfig {
        placement,
        width: 220,
        height: 112 + tb_h,
        draggable: true,
        drag_height: if no_titlebar { 112 + tb_h } else { 28 },
        position_key: Some("countdown_timer".into()),
    })?;

    if let Some(bd) = backdrop {
        panel.set_backdrop(bd);
    }
    if capture_excluded {
        panel.set_capture_excluded(true);
    }
    if opacity < 1.0 {
        panel.set_opacity(opacity);
    }

    // Background
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 220.0,
            height: (112 + tb_h) as f32,
            fill: Color::rgba(18, 18, 22, 255),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 23)),
            border_width: 1.0,
            interactive: false,
        },
    );

    // Title
    if !no_titlebar {
        panel.set_text(
            "title",
            TextElement {
                text: "Countdown".into(),
                x: 12.0,
                y: 8.0,
                font_size: 16.0,
                color: Color::rgba(232, 232, 237, 255),
                bold: true,
                ..Default::default()
            },
        );
    }

    // Close button
    panel.set_rect(
        "close_btn",
        RectElement {
            x: 194.0,
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
    panel.set_text(
        "close_x",
        TextElement {
            text: "×".into(),
            x: 199.0,
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
            y: sep_y,
            width: 196.0,
            height: 1.0,
            fill: Color::rgba(255, 255, 255, 18),
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            interactive: false,
        },
    );

    // Timer display
    panel.set_text(
        "timer",
        TextElement {
            text: format_time(INITIAL_SECS),
            x: 65.0,
            y: timer_y,
            font_size: 36.0,
            color: Color::rgba(232, 232, 237, 255),
            bold: true,
            font_family: Some("Consolas".into()),
            ..Default::default()
        },
    );

    // Start button
    panel.set_rect("btn_start", normal_rect(16.0, btn_y, 90.0));
    panel.set_text(
        "btn_start_text",
        TextElement {
            text: "Start".into(),
            x: 42.0,
            y: btn_text_y,
            font_size: 13.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );

    // Reset button
    panel.set_rect("btn_reset", normal_rect(114.0, btn_y, 90.0));
    panel.set_text(
        "btn_reset_text",
        TextElement {
            text: "Reset".into(),
            x: 140.0,
            y: btn_text_y,
            font_size: 13.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );

    panel.show();

    println!("Countdown panel visible at (850, 450). Press Ctrl+C to exit.");

    let panel_id = panel.id();
    let mut state = TimerState::Idle;
    let mut remaining_secs = INITIAL_SECS;
    let mut last_tick = Instant::now();
    let start_time = Instant::now();
    let mut prev_displayed_secs = INITIAL_SECS;
    let mut prev_state = TimerState::Idle;
    let mut prev_label = "Start";

    loop {
        while let Some(event) = ctx.poll_event() {
            match event {
                Event::ElementClicked {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => match key.as_str() {
                    "btn_start" => match state {
                        TimerState::Idle | TimerState::Paused => {
                            if remaining_secs == 0 {
                                remaining_secs = INITIAL_SECS;
                            }
                            state = TimerState::Running;
                            last_tick = Instant::now();
                        }
                        TimerState::Running => {
                            state = TimerState::Paused;
                        }
                    },
                    "btn_reset" => {
                        state = TimerState::Idle;
                        remaining_secs = INITIAL_SECS;
                    }
                    "close_btn" => return Ok(()),
                    _ => {}
                },
                Event::ElementHovered {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => match key.as_str() {
                    "btn_start" => {
                        panel.set_rect("btn_start", hover_rect(16.0, btn_y, 90.0));
                    }
                    "btn_reset" => {
                        panel.set_rect("btn_reset", hover_rect(114.0, btn_y, 90.0));
                    }
                    "close_btn" => {
                        panel.set_rect(
                            "close_btn",
                            RectElement {
                                x: 194.0,
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
                    _ => {}
                },
                Event::ElementLeft {
                    surface_id,
                    ref key,
                } if surface_id == panel_id => match key.as_str() {
                    "btn_start" => {
                        panel.set_rect("btn_start", normal_rect(16.0, btn_y, 90.0));
                    }
                    "btn_reset" => {
                        panel.set_rect("btn_reset", normal_rect(114.0, btn_y, 90.0));
                    }
                    "close_btn" => {
                        panel.set_rect(
                            "close_btn",
                            RectElement {
                                x: 194.0,
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
                    _ => {}
                },
                _ => {}
            }
        }

        // Timer tick
        if state == TimerState::Running && last_tick.elapsed() >= Duration::from_secs(1) {
            last_tick = Instant::now();
            remaining_secs = remaining_secs.saturating_sub(1);
            if remaining_secs == 0 {
                state = TimerState::Idle;
            }
        }

        // Update timer display only when something changed
        let elapsed_ms = start_time.elapsed().as_millis();
        if remaining_secs != prev_displayed_secs || state != prev_state {
            panel.set_text(
                "timer",
                TextElement {
                    text: format_time(remaining_secs),
                    x: 65.0,
                    y: timer_y,
                    font_size: 36.0,
                    color: timer_color(remaining_secs, &state, elapsed_ms),
                    bold: true,
                    font_family: Some("Consolas".into()),
                    ..Default::default()
                },
            );
            prev_displayed_secs = remaining_secs;
        } else if state == TimerState::Running && remaining_secs <= 10 {
            // Pulsing red animation — update at frame rate only in final 10s
            panel.set_text(
                "timer",
                TextElement {
                    text: format_time(remaining_secs),
                    x: 65.0,
                    y: timer_y,
                    font_size: 36.0,
                    color: timer_color(remaining_secs, &state, elapsed_ms),
                    bold: true,
                    font_family: Some("Consolas".into()),
                    ..Default::default()
                },
            );
        }

        // Update start button label only when state changes
        let start_label = if state == TimerState::Running {
            "Pause"
        } else {
            "Start"
        };
        if start_label != prev_label || state != prev_state {
            panel.set_text(
                "btn_start_text",
                TextElement {
                    text: start_label.into(),
                    x: 42.0,
                    y: btn_text_y,
                    font_size: 13.0,
                    color: Color::rgba(232, 232, 237, 255),
                    ..Default::default()
                },
            );
            prev_label = start_label;
        }

        prev_state = state;

        thread::sleep(Duration::from_millis(16));
    }
}
