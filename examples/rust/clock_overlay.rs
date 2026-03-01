//! Demo: floating clock overlay
//!
//! A minimal live-updating draggable clock showing the current time and date.
//! Demonstrates: Panel creation, draggable surface, text elements, timed update loop,
//! bottom-right positioning, design system tokens.
//!
//! Run on Windows: cargo run -p winpane --example clock_overlay

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

use std::thread;
use std::time::Duration;

use windows::Win32::System::SystemInformation::GetLocalTime;
use winpane::{Color, Context, PanelConfig, Placement, RectElement, TextElement};

fn get_local_time() -> (String, String) {
    // SAFETY: GetLocalTime is always safe to call; returns local calendar time.
    let st = unsafe { GetLocalTime() };
    let time_str = format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond);
    let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let date_str = format!(
        "{} {} {}",
        days[st.wDayOfWeek as usize],
        months[(st.wMonth - 1) as usize],
        st.wDay
    );
    (time_str, date_str)
}

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    // Bottom-right placement assuming 1920×1080 primary monitor
    let panel = ctx.create_panel(PanelConfig {
        placement: Placement::Position { x: 1750, y: 972 },
        width: 150,
        height: 88,
        draggable: true,
        drag_height: 28,
    })?;

    // Glass background with rounded corners
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 150.0,
            height: 88.0,
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            interactive: false,
        },
    );

    // Title bar in drag region
    panel.set_rect(
        "title_bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 150.0,
            height: 28.0,
            fill: Color::rgba(28, 28, 33, 255),
            corner_radius: 10.0,
            ..Default::default()
        },
    );
    panel.set_text(
        "title",
        TextElement {
            x: 8.0,
            y: 6.0,
            text: "Clock".into(),
            font_size: 13.0,
            color: Color::rgba(148, 148, 160, 255),
            bold: true,
            ..Default::default()
        },
    );

    panel.show();

    println!("winpane clock_overlay: floating clock at bottom-right.");
    println!("Updates every second. Press Ctrl+C to exit.");

    loop {
        let (time, date) = get_local_time();

        panel.set_text(
            "time",
            TextElement {
                text: time,
                x: 16.0,
                y: 36.0,
                font_size: 28.0,
                color: Color::rgba(232, 232, 237, 255),
                ..Default::default()
            },
        );

        panel.set_text(
            "date",
            TextElement {
                text: date,
                x: 16.0,
                y: 68.0,
                font_size: 12.0,
                color: Color::rgba(148, 148, 160, 204),
                ..Default::default()
            },
        );

        thread::sleep(Duration::from_secs(1));
    }
}
