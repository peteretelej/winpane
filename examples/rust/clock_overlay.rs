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
use winpane::{Anchor, Color, Context, PanelConfig, Placement, RectElement, TextElement};

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
    // ── CLI flags ──────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: clock_overlay [OPTIONS]");
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

    let placement = if let Some((x, y)) = explicit_position {
        Placement::Position { x, y }
    } else {
        Placement::Monitor {
            index: monitor_index,
            anchor: Anchor::BottomRight,
            margin: 20,
        }
    };

    let ctx = Context::new()?;

    let panel = ctx.create_panel(PanelConfig {
        placement,
        width: 150,
        height: 60 + tb_h,
        draggable: true,
        drag_height: if no_titlebar { 60 + tb_h } else { 28 },
        position_key: Some("clock_overlay".into()),
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

    // Glass background with rounded corners
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 150.0,
            height: (60 + tb_h) as f32,
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            interactive: false,
        },
    );

    // Title bar in drag region
    if !no_titlebar {
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
    }

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
                y: tb_h as f32 + 8.0,
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
                y: tb_h as f32 + 40.0,
                font_size: 12.0,
                color: Color::rgba(148, 148, 160, 204),
                ..Default::default()
            },
        );

        thread::sleep(Duration::from_secs(1));
    }
}
