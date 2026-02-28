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

#[derive(PartialEq)]
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
            // Pulsing red: sin wave alpha 128–255
            let phase = (elapsed_ms as f64 / 500.0) * std::f64::consts::PI;
            let alpha = 128.0 + 127.0 * phase.sin();
            Color::rgba(239, 68, 68, alpha as u8)
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
    use winpane::{Color, Context, Event, PanelConfig, RectElement, TextElement};
    use std::thread;
    use std::time::{Duration, Instant};

    let ctx = Context::new()?;

    let panel = ctx.create_panel(PanelConfig {
        x: 850,
        y: 450,
        width: 220,
        height: 140,
        draggable: true,
        drag_height: 28,
    })?;

    // Background
    panel.set_rect(
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

    // Title
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
            y: 28.0,
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
            y: 38.0,
            font_size: 36.0,
            color: Color::rgba(232, 232, 237, 255),
            bold: true,
            font_family: Some("Consolas".into()),
            ..Default::default()
        },
    );

    // Start button
    panel.set_rect("btn_start", normal_rect(16.0, 92.0, 90.0));
    panel.set_text(
        "btn_start_text",
        TextElement {
            text: "Start".into(),
            x: 42.0,
            y: 100.0,
            font_size: 13.0,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );

    // Reset button
    panel.set_rect("btn_reset", normal_rect(114.0, 92.0, 90.0));
    panel.set_text(
        "btn_reset_text",
        TextElement {
            text: "Reset".into(),
            x: 140.0,
            y: 100.0,
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
                        panel.set_rect("btn_start", hover_rect(16.0, 92.0, 90.0));
                    }
                    "btn_reset" => {
                        panel.set_rect("btn_reset", hover_rect(114.0, 92.0, 90.0));
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
                        panel.set_rect("btn_start", normal_rect(16.0, 92.0, 90.0));
                    }
                    "btn_reset" => {
                        panel.set_rect("btn_reset", normal_rect(114.0, 92.0, 90.0));
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
            if remaining_secs > 0 {
                remaining_secs -= 1;
            }
            if remaining_secs == 0 {
                state = TimerState::Idle;
            }
        }

        // Update timer display every frame
        let elapsed_ms = start_time.elapsed().as_millis();
        panel.set_text(
            "timer",
            TextElement {
                text: format_time(remaining_secs),
                x: 65.0,
                y: 38.0,
                font_size: 36.0,
                color: timer_color(remaining_secs, &state, elapsed_ms),
                bold: true,
                font_family: Some("Consolas".into()),
                ..Default::default()
            },
        );

        // Update start button label
        let start_label = if state == TimerState::Running {
            "Pause"
        } else {
            "Start"
        };
        panel.set_text(
            "btn_start_text",
            TextElement {
                text: start_label.into(),
                x: 42.0,
                y: 100.0,
                font_size: 13.0,
                color: Color::rgba(232, 232, 237, 255),
                ..Default::default()
            },
        );

        thread::sleep(Duration::from_millis(16));
    }
}
