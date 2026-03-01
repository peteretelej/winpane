//! Demo: CGM glucose monitor overlay
//!
//! Displays a live continuous glucose monitor (CGM) reading as a compact draggable panel.
//! Connects to a Nightscout instance when NIGHTSCOUT_URL is set (optionally
//! with NIGHTSCOUT_TOKEN), otherwise runs in simulated mode with random-walk
//! glucose values.
//!
//! Demonstrates: Panel creation, draggable surface, dynamic background color, text elements,
//! timed polling, environment-driven configuration, design system tokens, CLI flags.
//!
//! Run on Windows: cargo run -p winpane --example glucose_monitor
//!
//! Flags:
//!   --demo           — cycle through preset glucose values (ignores Nightscout)
//!   --unit mmol      — display in mmol/L instead of mg/dL
//!   --help           — print usage and exit
//!
//! Environment variables (optional):
//!   NIGHTSCOUT_URL   — base URL of your Nightscout site (e.g. https://my.ns.site)
//!   NIGHTSCOUT_TOKEN — API token for authenticated access

// ── winpane design tokens ──────────────────────────────────────
// Surface base:   rgb(18, 18, 22)  Glass: a=228  Solid: a=255  Muted: a=242
// Elevated:       rgb(28, 28, 33)  Interactive:  rgba(38, 38, 44, 255)
// Border:         rgba(255,255,255, 18)     Hover:       rgba(48, 48, 56, 255)
// Text primary:   rgba(232, 232, 237, 255)  Secondary:   rgba(148, 148, 160, 255)
// Accent:         rgba(82, 139, 255, 255)   Accent hover:rgba(110, 160, 255, 255)
// Success:        rgba(52, 211, 153, 255)   Warning:     rgba(251, 191, 36, 255)
// Danger:         rgba(239, 68, 68, 255)    Radius: 10/6 px
// ────────────────────────────────────────────────────────────────

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::thread;
use std::time::{Duration, Instant};

use winpane::{Anchor, Color, Context, PanelConfig, Placement, RectElement, TextElement};

/// A single glucose reading with value, trend direction, and fetch timestamp.
struct GlucoseReading {
    sgv: u32,
    direction: String,
    timestamp: Instant,
}

/// Returns a background color based on the glucose range.
///
/// - 70–180 (normal): green-tinted dark
/// - 181–250 (high): amber-tinted dark
/// - <70 or >250 (urgent): red-tinted dark
fn bg_color_for_sgv(sgv: u32) -> Color {
    match sgv {
        70..=180 => Color::rgba(18, 40, 30, 228),
        181..=250 => Color::rgba(40, 36, 18, 228),
        _ => Color::rgba(40, 18, 18, 228),
    }
}

/// Returns an arrow/text color based on the glucose range.
fn arrow_color_for_sgv(sgv: u32) -> Color {
    match sgv {
        70..=180 => Color::rgba(52, 211, 153, 255),
        55..=69 | 181..=250 => Color::rgba(251, 191, 36, 255),
        _ => Color::rgba(239, 68, 68, 255),
    }
}

/// Returns a human-readable range label for the glucose value.
fn range_label(sgv: u32) -> &'static str {
    match sgv {
        70..=180 => "IN RANGE",
        181..=250 => "HIGH",
        251.. => "URGENT HIGH",
        55..=69 => "LOW",
        _ => "URGENT LOW",
    }
}

const MGDL_TO_MMOL: f64 = 18.0182;

/// Formats a glucose value in the requested unit.
fn format_glucose(sgv: u32, mmol: bool) -> String {
    if mmol {
        format!("{:.1}", sgv as f64 / MGDL_TO_MMOL)
    } else {
        format!("{sgv}")
    }
}

/// Returns the unit label string.
fn unit_label(mmol: bool) -> &'static str {
    if mmol { "mmol/L" } else { "mg/dL" }
}

/// Maps a Nightscout trend direction string to a Unicode arrow.
fn direction_to_arrow(direction: &str) -> &str {
    match direction {
        "DoubleUp" => "⇈",
        "SingleUp" => "↑",
        "FortyFiveUp" => "↗",
        "Flat" => "→",
        "FortyFiveDown" => "↘",
        "SingleDown" => "↓",
        "DoubleDown" => "⇊",
        _ => "?",
    }
}

/// Formats elapsed time since the reading and picks an appropriate text color.
///
/// NOTE: staleness is measured from the last *fetch* time, not the CGM sensor
/// reading timestamp. A production app should use the CGM `dateString` field.
fn staleness_text(elapsed: Duration) -> (String, Color) {
    let secs = elapsed.as_secs();
    let color = if secs > 15 * 60 {
        // Stale: >15 minutes
        Color::rgba(239, 68, 68, 255)
    } else {
        Color::rgba(148, 148, 160, 204)
    };

    let text = if secs < 60 {
        "just now".to_string()
    } else {
        format!("{} min ago", secs / 60)
    };

    (text, color)
}

/// Fetches the latest glucose entry from a Nightscout server.
fn fetch_nightscout(url: &str, token: Option<&str>) -> Option<GlucoseReading> {
    let mut request_url = format!("{url}/api/v1/entries/current.json");
    if let Some(t) = token {
        request_url.push_str(&format!("?token={t}"));
    }
    let mut response = ureq::get(&request_url).call().ok()?;
    let body = response.body_mut().read_to_string().ok()?;
    let entries: serde_json::Value = serde_json::from_str(&body).ok()?;
    let entry = entries.get(0)?;
    let sgv = entry.get("sgv")?.as_u64()? as u32;
    let direction = entry
        .get("direction")?
        .as_str()
        .unwrap_or("NONE")
        .to_string();
    Some(GlucoseReading {
        sgv,
        direction,
        timestamp: Instant::now(),
    })
}

/// Produces a simulated glucose reading using a deterministic random walk.
/// Uses `DefaultHasher` seeded with the current time for pseudo-randomness.
fn simulate_reading(prev_sgv: u32) -> GlucoseReading {
    let now = Instant::now();
    let nanos = now.elapsed().as_nanos() ^ (prev_sgv as u128);
    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    let hash = hasher.finish();

    // Random delta in -20..=20
    let delta = (hash % 41) as i32 - 20;
    let new_sgv = (prev_sgv as i32 + delta).clamp(40, 350) as u32;

    let direction = match delta {
        d if d > 10 => "SingleUp",
        d if d > 5 => "FortyFiveUp",
        d if d > -5 => "Flat",
        d if d > -10 => "FortyFiveDown",
        _ => "SingleDown",
    }
    .to_string();

    GlucoseReading {
        sgv: new_sgv,
        direction,
        timestamp: Instant::now(),
    }
}

/// A single step in the demo sequence.
struct DemoStep {
    sgv: u32,
    direction: &'static str,
    hold_secs: u64,
}

const DEMO_SEQUENCE: &[DemoStep] = &[
    DemoStep {
        sgv: 110,
        direction: "Flat",
        hold_secs: 5,
    },
    DemoStep {
        sgv: 140,
        direction: "FortyFiveUp",
        hold_secs: 4,
    },
    DemoStep {
        sgv: 180,
        direction: "SingleUp",
        hold_secs: 4,
    },
    DemoStep {
        sgv: 220,
        direction: "SingleUp",
        hold_secs: 4,
    },
    DemoStep {
        sgv: 290,
        direction: "DoubleUp",
        hold_secs: 5,
    },
    DemoStep {
        sgv: 250,
        direction: "FortyFiveDown",
        hold_secs: 4,
    },
    DemoStep {
        sgv: 180,
        direction: "SingleDown",
        hold_secs: 4,
    },
    DemoStep {
        sgv: 110,
        direction: "FortyFiveDown",
        hold_secs: 4,
    },
    DemoStep {
        sgv: 80,
        direction: "SingleDown",
        hold_secs: 4,
    },
    DemoStep {
        sgv: 62,
        direction: "SingleDown",
        hold_secs: 5,
    },
    DemoStep {
        sgv: 45,
        direction: "DoubleDown",
        hold_secs: 5,
    },
    DemoStep {
        sgv: 70,
        direction: "FortyFiveUp",
        hold_secs: 4,
    },
];

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: glucose_monitor [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --demo              Cycle through preset glucose values");
        println!("  --unit mmol         Display in mmol/L (default: mg/dL)");
        println!("  --no-titlebar       Hide title bar, drag anywhere");
        println!("  --opacity <0.0-1.0> Surface opacity");
        println!("  --backdrop <type>   Backdrop: mica, acrylic");
        println!("  --capture-excluded  Hide from screenshots");
        println!("  --position <X,Y>    Explicit position");
        println!("  --monitor <N>       Monitor index (0=primary)");
        std::process::exit(0);
    }
    let demo_mode = args.iter().any(|a| a == "--demo");
    let unit_mmol = args
        .iter()
        .position(|a| a == "--unit")
        .and_then(|i| args.get(i + 1).map(String::as_str))
        .map(|s| s == "mmol")
        .unwrap_or(false);

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
    let explicit_monitor = args.iter().any(|a| a == "--monitor");

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
        width: 170,
        height: 80 + tb_h,
        draggable: true,
        drag_height: if no_titlebar { 80 + tb_h } else { 28 },
        position_key: if explicit_position.is_some() || explicit_monitor {
            None
        } else {
            Some("glucose_monitor".into())
        },
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

    // Initial glass background
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 170.0,
            height: (80 + tb_h) as f32,
            fill: bg_color_for_sgv(160),
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
                width: 170.0,
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
                text: "Glucose".into(),
                font_size: 13.0,
                color: Color::rgba(148, 148, 160, 255),
                bold: true,
                ..Default::default()
            },
        );
    }

    panel.show();

    let nightscout_url = std::env::var("NIGHTSCOUT_URL").ok();
    let nightscout_token = std::env::var("NIGHTSCOUT_TOKEN").ok();

    let poll_interval = if nightscout_url.is_some() {
        Duration::from_secs(5 * 60) // 5 minutes for live data
    } else {
        Duration::from_secs(30) // 30 seconds for simulation
    };

    if demo_mode {
        println!("winpane glucose_monitor: demo mode (~52s cycle).");
    } else if nightscout_url.is_some() {
        println!("winpane glucose_monitor: polling Nightscout every 5 min.");
    } else {
        println!("winpane glucose_monitor: simulated mode (set NIGHTSCOUT_URL for live data).");
    }
    println!("Press Ctrl+C to exit.");

    let mut last_poll = Instant::now() - poll_interval; // force immediate first poll
    let mut current_reading = GlucoseReading {
        sgv: 160,
        direction: "Flat".to_string(),
        timestamp: Instant::now(),
    };

    // Demo mode state
    let mut demo_index: usize = 0;
    let mut demo_step_start = Instant::now();

    loop {
        if demo_mode {
            let step = &DEMO_SEQUENCE[demo_index];
            if demo_step_start.elapsed() >= Duration::from_secs(step.hold_secs) {
                demo_index = (demo_index + 1) % DEMO_SEQUENCE.len();
                demo_step_start = Instant::now();
            }
            let step = &DEMO_SEQUENCE[demo_index];
            current_reading = GlucoseReading {
                sgv: step.sgv,
                direction: step.direction.to_string(),
                timestamp: Instant::now(),
            };
        } else if last_poll.elapsed() >= poll_interval {
            if let Some(ref url) = nightscout_url {
                if let Some(reading) = fetch_nightscout(url, nightscout_token.as_deref()) {
                    current_reading = reading;
                }
            } else {
                current_reading = simulate_reading(current_reading.sgv);
            }
            last_poll = Instant::now();
        }

        let sgv = current_reading.sgv;

        // Update background color based on glucose range
        panel.set_rect(
            "bg",
            RectElement {
                x: 0.0,
                y: 0.0,
                width: 170.0,
                height: (80 + tb_h) as f32,
                fill: bg_color_for_sgv(sgv),
                corner_radius: 10.0,
                border_color: Some(Color::rgba(255, 255, 255, 18)),
                border_width: 1.0,
                interactive: false,
            },
        );

        // Update reading text: "{glucose} {arrow}"
        let arrow = direction_to_arrow(&current_reading.direction);
        panel.set_text(
            "reading",
            TextElement {
                text: format!("{} {}", format_glucose(sgv, unit_mmol), arrow),
                x: 12.0,
                y: tb_h as f32 + 6.0,
                font_size: 28.0,
                color: arrow_color_for_sgv(sgv),
                bold: true,
                font_family: Some("Consolas".to_string()),
                ..Default::default()
            },
        );

        // Unit label
        panel.set_text(
            "unit",
            TextElement {
                text: unit_label(unit_mmol).to_string(),
                x: 120.0,
                y: tb_h as f32 + 7.0,
                font_size: 11.0,
                color: Color::rgba(148, 148, 160, 255),
                ..Default::default()
            },
        );

        // Range label
        panel.set_text(
            "range",
            TextElement {
                text: range_label(sgv).to_string(),
                x: 12.0,
                y: tb_h as f32 + 38.0,
                font_size: 11.0,
                color: arrow_color_for_sgv(sgv),
                bold: true,
                ..Default::default()
            },
        );

        // Update staleness text
        let elapsed = current_reading.timestamp.elapsed();
        let (stale_text, stale_color) = staleness_text(elapsed);
        panel.set_text(
            "staleness",
            TextElement {
                text: stale_text,
                x: 12.0,
                y: tb_h as f32 + 52.0,
                font_size: 12.0,
                color: stale_color,
                ..Default::default()
            },
        );

        thread::sleep(Duration::from_secs(1));
    }
}
