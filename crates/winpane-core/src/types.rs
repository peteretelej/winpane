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
    pub interactive: bool,
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
            interactive: false,
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
    pub interactive: bool,
}

impl Default for RectElement {
    fn default() -> Self {
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            fill: Color::TRANSPARENT,
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            interactive: false,
        }
    }
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
    pub interactive: bool,
}

// --- DrawOp ---

/// Low-level drawing operations for the custom draw escape hatch.
/// Accumulated by the FFI canvas and sent as a batch to the engine.
#[derive(Debug, Clone)]
pub enum DrawOp {
    Clear(Color),
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    StrokeRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        stroke_width: f32,
    },
    DrawText {
        x: f32,
        y: f32,
        text: String,
        font_size: f32,
        color: Color,
    },
    DrawLine {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        stroke_width: f32,
    },
    FillEllipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        color: Color,
    },
    StrokeEllipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        color: Color,
        stroke_width: f32,
    },
    DrawImage {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        rgba: Vec<u8>,
        img_width: u32,
        img_height: u32,
    },
    FillRoundedRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: Color,
    },
    StrokeRoundedRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: Color,
        stroke_width: f32,
    },
}

// --- HudConfig ---

#[derive(Debug, Clone)]
pub struct HudConfig {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// --- PanelConfig ---

#[derive(Debug, Clone, Default)]
pub struct PanelConfig {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub draggable: bool,
    /// Logical pixels from top of panel that act as a drag handle.
    pub drag_height: u32,
}

// --- TrayConfig ---

#[derive(Debug, Clone)]
pub struct TrayConfig {
    /// RGBA8 pixel data for the tray icon.
    pub icon_rgba: Vec<u8>,
    pub icon_width: u32,
    pub icon_height: u32,
    /// Tooltip text (max 127 chars due to NOTIFYICONDATAW limit).
    pub tooltip: String,
}

// --- TrayId ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrayId(pub u64);

// --- PipConfig ---

#[derive(Debug, Clone)]
pub struct PipConfig {
    /// Raw HWND value of the source window to thumbnail.
    pub source_hwnd: isize,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// --- SourceRect ---

/// Defines a crop region on the PiP source window.
#[derive(Debug, Clone, Copy)]
pub struct SourceRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

// --- Anchor ---

/// Anchor point on a target window for surface positioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

// --- Backdrop ---

/// DWM backdrop material for a surface window.
/// Requires Windows 11 22H2+ (build 22621). Silent no-op on older versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    None,
    Mica,
    Acrylic,
}

// --- Event ---

#[derive(Debug, Clone)]
pub enum Event {
    ElementClicked { surface_id: SurfaceId, key: String },
    ElementHovered { surface_id: SurfaceId, key: String },
    ElementLeft { surface_id: SurfaceId, key: String },
    TrayClicked { button: MouseButton },
    TrayMenuItemClicked { id: u32 },
    PipSourceClosed { surface_id: SurfaceId },
    AnchorTargetClosed { surface_id: SurfaceId },
    DeviceRecovered,
}

// --- MouseButton ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// --- MenuItem ---

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: u32,
    pub label: String,
    pub enabled: bool,
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
    UnsupportedOperation(String),
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
            Error::UnsupportedOperation(msg) => write!(f, "unsupported operation: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

// --- SurfaceId ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u64);

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
