//! Demo: system monitor overlay
//!
//! A live-updating draggable panel showing real CPU usage, memory consumption, and system uptime.
//! Demonstrates: Panel creation, draggable surface, Win32 performance APIs, colored progress bars,
//! design system tokens, 2-second update loop.
//!
//! Run on Windows: cargo run -p winpane --example system_monitor

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

use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::SystemInformation::{
    GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows::Win32::System::Threading::GetSystemTimes;
use winpane::{Color, Context, PanelConfig, RectElement, TextElement};

// ── CPU helpers ────────────────────────────────────────────────

fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
}

struct CpuSample {
    idle: u64,
    kernel: u64,
    user: u64,
}

fn sample_cpu() -> Option<CpuSample> {
    let mut idle = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: pointers are valid mutable references; Win32 API is thread-safe.
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
    Some(CpuSample {
        idle: filetime_to_u64(idle),
        kernel: filetime_to_u64(kernel),
        user: filetime_to_u64(user),
    })
}

fn cpu_percent(prev: &CpuSample, curr: &CpuSample) -> f64 {
    let idle_delta = curr.idle.saturating_sub(prev.idle);
    let kernel_delta = curr.kernel.saturating_sub(prev.kernel);
    let user_delta = curr.user.saturating_sub(prev.user);
    let total = kernel_delta + user_delta;
    if total == 0 {
        return 0.0;
    }
    let active = total.saturating_sub(idle_delta);
    ((active as f64 / total as f64) * 100.0).clamp(0.0, 100.0)
}

// ── Memory helper ──────────────────────────────────────────────

struct MemInfo {
    used_gb: f64,
    percent: u32,
}

fn get_memory_info() -> Option<MemInfo> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: dwLength is set; pointer is a valid mutable reference.
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    let used = status.ullTotalPhys.saturating_sub(status.ullAvailPhys);
    Some(MemInfo {
        used_gb: used as f64 / (1024.0 * 1024.0 * 1024.0),
        percent: status.dwMemoryLoad,
    })
}

// ── Uptime helper ──────────────────────────────────────────────

fn format_uptime() -> String {
    // SAFETY: GetTickCount64 has no preconditions.
    let ms = unsafe { GetTickCount64() };
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

// ── Bar color ──────────────────────────────────────────────────

fn bar_color(percent: u32) -> Color {
    match percent {
        0..=59 => Color::rgba(52, 211, 153, 255), // success (green)
        60..=85 => Color::rgba(251, 191, 36, 255), // warning (yellow)
        _ => Color::rgba(239, 68, 68, 255),       // danger (red)
    }
}

// ── Scene setup ────────────────────────────────────────────────

fn setup_static_scene(panel: &winpane::Panel) {
    // Background
    panel.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 180.0,
            height: 108.0,
            fill: Color::rgba(18, 18, 22, 228),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            interactive: false,
        },
    );

    // Row labels (secondary text color)
    let label_color = Color::rgba(148, 148, 160, 255);
    for (key, text, y) in [
        ("cpu_label", "CPU", 36.0),
        ("mem_label", "MEM", 58.0),
        ("up_label", "UP", 80.0),
    ] {
        panel.set_text(
            key,
            TextElement {
                text: text.into(),
                x: 12.0,
                y,
                font_size: 12.0,
                color: label_color,
                bold: true,
                ..Default::default()
            },
        );
    }

    // Bar track backgrounds (elevated color)
    let track_color = Color::rgba(28, 28, 33, 255);
    panel.set_rect(
        "cpu_bar_bg",
        RectElement {
            x: 92.0,
            y: 38.0,
            width: 76.0,
            height: 10.0,
            fill: track_color,
            corner_radius: 3.0,
            ..Default::default()
        },
    );
    panel.set_rect(
        "mem_bar_bg",
        RectElement {
            x: 110.0,
            y: 60.0,
            width: 58.0,
            height: 10.0,
            fill: track_color,
            corner_radius: 3.0,
            ..Default::default()
        },
    );

    // Title bar in drag region
    panel.set_rect(
        "title_bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 180.0,
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
            text: "System Monitor".into(),
            font_size: 13.0,
            color: Color::rgba(148, 148, 160, 255),
            bold: true,
            ..Default::default()
        },
    );
}

// ── Dynamic updates ────────────────────────────────────────────

fn update_cpu_display(panel: &winpane::Panel, pct: f64) {
    let pct_u32 = pct.round() as u32;
    panel.set_text(
        "cpu_pct",
        TextElement {
            text: format!("{pct_u32}%"),
            x: 52.0,
            y: 36.0,
            font_size: 12.0,
            color: Color::rgba(232, 232, 237, 255),
            font_family: Some("Consolas".into()),
            ..Default::default()
        },
    );
    panel.set_rect(
        "cpu_bar",
        RectElement {
            x: 92.0,
            y: 38.0,
            width: (76.0 * pct / 100.0) as f32,
            height: 10.0,
            fill: bar_color(pct_u32),
            corner_radius: 3.0,
            ..Default::default()
        },
    );
}

fn update_memory_display(panel: &winpane::Panel, info: &MemInfo) {
    panel.set_text(
        "mem_val",
        TextElement {
            text: format!("{:.1} GB", info.used_gb),
            x: 48.0,
            y: 58.0,
            font_size: 12.0,
            color: Color::rgba(232, 232, 237, 255),
            font_family: Some("Consolas".into()),
            ..Default::default()
        },
    );
    panel.set_rect(
        "mem_bar",
        RectElement {
            x: 110.0,
            y: 60.0,
            width: (58.0 * info.percent as f64 / 100.0) as f32,
            height: 10.0,
            fill: bar_color(info.percent),
            corner_radius: 3.0,
            ..Default::default()
        },
    );
}

fn update_uptime_display(panel: &winpane::Panel) {
    panel.set_text(
        "up_val",
        TextElement {
            text: format_uptime(),
            x: 48.0,
            y: 80.0,
            font_size: 12.0,
            color: Color::rgba(232, 232, 237, 255),
            font_family: Some("Consolas".into()),
            ..Default::default()
        },
    );
}

// ── Entry point ────────────────────────────────────────────────

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let panel = ctx.create_panel(PanelConfig {
        x: 20,
        y: 20,
        width: 180,
        height: 108,
        draggable: true,
        drag_height: 28,
    })?;

    setup_static_scene(&panel);
    panel.show();

    println!("winpane system_monitor: live system stats at top-left.");
    println!("Updates every 2 seconds. Press Ctrl+C to exit.");

    let mut prev_cpu = sample_cpu().expect("failed to read initial CPU times");

    loop {
        thread::sleep(Duration::from_secs(2));

        if let Some(curr_cpu) = sample_cpu() {
            let pct = cpu_percent(&prev_cpu, &curr_cpu);
            update_cpu_display(&panel, pct);
            prev_cpu = curr_cpu;
        }

        if let Some(mem) = get_memory_info() {
            update_memory_display(&panel, &mem);
        }

        update_uptime_display(&panel);
    }
}
