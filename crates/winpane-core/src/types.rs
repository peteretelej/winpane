use std::fmt;

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

// --- Color ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    /// Converts to premultiplied alpha floats for Direct2D.
    /// Formula: component_f32 = (component / 255.0) * (a / 255.0), alpha = a / 255.0
    #[cfg(target_os = "windows")]
    pub(crate) fn to_d2d_premultiplied(&self) -> D2D1_COLOR_F {
        let a = self.a as f32 / 255.0;
        D2D1_COLOR_F {
            r: (self.r as f32 / 255.0) * a,
            g: (self.g as f32 / 255.0) * a,
            b: (self.b as f32 / 255.0) * a,
            a,
        }
    }
}

// --- TextElement ---

#[derive(Debug, Clone)]
pub struct TextElement {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    pub color: Color,
    pub font_family: Option<String>,
    pub bold: bool,
    pub italic: bool,
}

impl Default for TextElement {
    fn default() -> Self {
        TextElement {
            text: String::new(),
            x: 0.0,
            y: 0.0,
            font_size: 14.0,
            color: Color::WHITE,
            font_family: None,
            bold: false,
            italic: false,
        }
    }
}

// --- RectElement ---

#[derive(Debug, Clone)]
pub struct RectElement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fill: Color,
    pub corner_radius: f32,
    pub border_color: Option<Color>,
    pub border_width: f32,
}

// --- ImageElement ---

#[derive(Debug, Clone)]
pub struct ImageElement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// RGBA8 premultiplied pixel data, row-major.
    /// The renderer converts to BGRA (D2D native) internally.
    pub data: Vec<u8>,
    pub data_width: u32,
    pub data_height: u32,
}

// --- HudConfig ---

#[derive(Debug, Clone)]
pub struct HudConfig {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// --- Error ---

#[derive(Debug)]
pub enum Error {
    WindowCreation(String),
    DeviceCreation(String),
    SwapChainCreation(String),
    RenderError(String),
    ThreadSpawnFailed,
    SurfaceNotFound,
    Shutdown,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::WindowCreation(msg) => write!(f, "window creation failed: {msg}"),
            Error::DeviceCreation(msg) => write!(f, "device creation failed: {msg}"),
            Error::SwapChainCreation(msg) => write!(f, "swap chain creation failed: {msg}"),
            Error::RenderError(msg) => write!(f, "render error: {msg}"),
            Error::ThreadSpawnFailed => write!(f, "failed to spawn engine thread"),
            Error::SurfaceNotFound => write!(f, "surface not found"),
            Error::Shutdown => write!(f, "engine has shut down"),
        }
    }
}

impl std::error::Error for Error {}

// --- SurfaceId ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SurfaceId(pub u64);

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_rgba() {
        let c = Color::rgba(255, 128, 0, 200);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 200);
    }

    #[test]
    fn color_constants() {
        assert_eq!(
            Color::WHITE,
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
        assert_eq!(
            Color::BLACK,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            }
        );
        assert_eq!(
            Color::TRANSPARENT,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0
            }
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn color_premultiplied_conversion() {
        // 50% transparent red: r=255, a=128
        // Premultiplied: r = (255/255) * (128/255) = 0.502
        let c = Color::rgba(255, 0, 0, 128);
        let d2d = c.to_d2d_premultiplied();
        assert!((d2d.r - 0.502).abs() < 0.01);
        assert!((d2d.g - 0.0).abs() < 0.01);
        assert!((d2d.b - 0.0).abs() < 0.01);
        assert!((d2d.a - 0.502).abs() < 0.01);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn color_opaque_premultiplied() {
        // Fully opaque: premultiplied = straight
        let c = Color::rgba(100, 200, 50, 255);
        let d2d = c.to_d2d_premultiplied();
        assert!((d2d.r - 100.0 / 255.0).abs() < 0.01);
        assert!((d2d.g - 200.0 / 255.0).abs() < 0.01);
        assert!((d2d.b - 50.0 / 255.0).abs() < 0.01);
        assert!((d2d.a - 1.0).abs() < 0.01);
    }
}
