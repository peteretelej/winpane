//! Demo: color picker overlay
//!
//! A live pixel color sampler that follows the cursor, showing the color
//! as a swatch, hex value, and RGB decimal.
//! Demonstrates: Hud creation, cursor tracking via set_position, Win32 GDI
//! pixel sampling, fast 50ms update loop.
//!
//! Run on Windows: cargo run -p winpane --example color_picker

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

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use winpane::{Color, Context, HudConfig, RectElement, TextElement};

/// Sample the pixel colour at screen coordinates (`x`, `y`).
///
/// Returns `None` when `GetPixel` returns `CLR_INVALID` (0xFFFF_FFFF).
fn sample_pixel(x: i32, y: i32) -> Option<(u8, u8, u8)> {
    // SAFETY: GetDC(None) returns the screen DC which is always valid.
    // ReleaseDC releases that same DC. GetPixel reads one pixel.
    unsafe {
        let hdc = GetDC(None);
        let color = GetPixel(hdc, x, y);
        ReleaseDC(None, hdc);

        // CLR_INVALID means the point is outside any visible area.
        if color.0 == 0xFFFF_FFFF {
            return None;
        }

        // COLORREF is 0x00BBGGRR (BGR, not RGB).
        let r = (color.0 & 0xFF) as u8;
        let g = ((color.0 >> 8) & 0xFF) as u8;
        let b = ((color.0 >> 16) & 0xFF) as u8;
        Some((r, g, b))
    }
}

/// Return the current cursor position in screen coordinates.
fn get_cursor_pos() -> Option<(i32, i32)> {
    let mut pt = POINT::default();
    // SAFETY: GetCursorPos writes into our stack-allocated POINT.
    unsafe { GetCursorPos(&mut pt).ok()? };
    Some((pt.x, pt.y))
}

/// Set up the static parts of the scene that never change.
fn setup_static_scene(hud: &winpane::Hud) {
    // Muted glass background
    hud.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 160.0,
            height: 80.0,
            fill: Color::rgba(18, 18, 22, 242),
            corner_radius: 10.0,
            border_color: Some(Color::rgba(255, 255, 255, 18)),
            border_width: 1.0,
            ..Default::default()
        },
    );

    // Static "sRGB" label
    hud.set_text(
        "label",
        TextElement {
            text: "sRGB".into(),
            x: 108.0,
            y: 48.0,
            font_size: 11.0,
            color: Color::rgba(148, 148, 160, 255),
            ..Default::default()
        },
    );
}

/// Update the dynamic parts of the scene with the sampled colour.
fn update_display(hud: &winpane::Hud, r: u8, g: u8, b: u8) {
    // Colour swatch
    hud.set_rect(
        "swatch",
        RectElement {
            x: 12.0,
            y: 6.0,
            width: 136.0,
            height: 32.0,
            fill: Color::rgb(r, g, b),
            corner_radius: 6.0,
            border_color: Some(Color::rgba(255, 255, 255, 30)),
            border_width: 1.0,
            ..Default::default()
        },
    );

    // Hex value
    hud.set_text(
        "hex",
        TextElement {
            text: format!("#{r:02X}{g:02X}{b:02X}"),
            x: 12.0,
            y: 46.0,
            font_size: 14.0,
            font_family: Some("Consolas".into()),
            bold: true,
            color: Color::rgba(232, 232, 237, 255),
            ..Default::default()
        },
    );

    // RGB decimal
    hud.set_text(
        "rgb",
        TextElement {
            text: format!("{r}, {g}, {b}"),
            x: 12.0,
            y: 64.0,
            font_size: 12.0,
            font_family: Some("Consolas".into()),
            color: Color::rgba(148, 148, 160, 255),
            ..Default::default()
        },
    );
}

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        x: 100,
        y: 100,
        width: 160,
        height: 80,
    })?;

    setup_static_scene(&hud);
    update_display(&hud, 0, 0, 0);
    hud.show();

    println!("winpane color_picker: live pixel sampler following your cursor.");
    println!("Move the mouse to sample colours. Press Ctrl+C to exit.");

    loop {
        thread::sleep(Duration::from_millis(50));

        let Some((cx, cy)) = get_cursor_pos() else {
            continue;
        };

        hud.set_position(cx + 20, cy + 20);

        let Some((r, g, b)) = sample_pixel(cx, cy) else {
            continue;
        };

        update_display(&hud, r, g, b);
    }
}
