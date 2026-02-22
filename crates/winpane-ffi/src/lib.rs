//! winpane-ffi: C ABI bindings for winpane.
//!
//! Produces winpane.dll (cdylib) with extern "C" functions consumable
//! from any language with C FFI support (C, C++, Go, Zig, C#, Python).

#![allow(clippy::missing_safety_doc)] // FFI functions document safety via C header

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;

// --- Thread-local error storage ---

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(msg: impl fmt::Display) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = CString::new(msg.to_string()).ok();
    });
}

/// Returns the last error message, or NULL if no error.
/// The returned pointer is valid until the next winpane call on the same thread.
#[no_mangle]
pub extern "C" fn winpane_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |s| s.as_ptr())
    })
}

// --- ffi_try! macro ---
//
// Wraps every extern "C" function body in catch_unwind + Result handling.
// Returns 0 on success, -1 on error (with last_error set), -2 on panic.

macro_rules! ffi_try {
    ($body:expr) => {{
        match std::panic::catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(Ok(())) => 0_i32,
            Ok(Err(e)) => {
                set_last_error(&e);
                -1_i32
            }
            Err(_) => {
                set_last_error("panic caught at FFI boundary");
                -2_i32
            }
        }
    }};
}

// Variant for functions that return a value through an out-pointer.
// The Ok branch yields the value; error paths use early return.
macro_rules! ffi_try_with {
    ($body:expr) => {{
        match std::panic::catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(Ok(val)) => val,
            Ok(Err(e)) => {
                set_last_error(&e);
                return -1_i32;
            }
            Err(_) => {
                set_last_error("panic caught at FFI boundary");
                return -2_i32;
            }
        }
    }};
}

// --- Null pointer validation helpers ---

fn require_non_null<T>(ptr: *const T, name: &str) -> Result<(), String> {
    if ptr.is_null() {
        Err(format!("{name} is null"))
    } else {
        Ok(())
    }
}

fn require_non_null_mut<T>(ptr: *mut T, name: &str) -> Result<(), String> {
    if ptr.is_null() {
        Err(format!("{name} is null"))
    } else {
        Ok(())
    }
}

// --- CStr helper ---

/// # Safety
/// `ptr` must point to a valid null-terminated C string if non-null.
unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("string pointer is null".into());
    }
    // Safety: caller guarantees valid null-terminated UTF-8
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|e| format!("invalid UTF-8: {e}"))
}

// ============================================================
// C-compatible type definitions
// ============================================================

/// WINPANE_CONFIG_VERSION: consumers set this in config structs.
pub const WINPANE_CONFIG_VERSION: u32 = 1;

// --- Color ---

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WinpaneColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl WinpaneColor {
    fn to_rust(&self) -> winpane::Color {
        winpane::Color::rgba(self.r, self.g, self.b, self.a)
    }
}

// --- Config structs (versioned) ---

#[repr(C)]
pub struct WinpaneHudConfig {
    pub version: u32,
    pub size: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl WinpaneHudConfig {
    fn to_rust(&self) -> Result<winpane::HudConfig, String> {
        if self.version != WINPANE_CONFIG_VERSION {
            return Err(format!(
                "unsupported config version {} (expected {})",
                self.version, WINPANE_CONFIG_VERSION
            ));
        }
        if (self.size as usize) < std::mem::size_of::<Self>() {
            return Err(format!(
                "config size {} too small (expected at least {})",
                self.size,
                std::mem::size_of::<Self>()
            ));
        }
        Ok(winpane::HudConfig {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        })
    }
}

#[repr(C)]
pub struct WinpanePanelConfig {
    pub version: u32,
    pub size: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub draggable: i32,
    pub drag_height: u32,
}

impl WinpanePanelConfig {
    fn to_rust(&self) -> Result<winpane::PanelConfig, String> {
        if self.version != WINPANE_CONFIG_VERSION {
            return Err(format!(
                "unsupported config version {} (expected {})",
                self.version, WINPANE_CONFIG_VERSION
            ));
        }
        if (self.size as usize) < std::mem::size_of::<Self>() {
            return Err(format!(
                "config size {} too small (expected at least {})",
                self.size,
                std::mem::size_of::<Self>()
            ));
        }
        Ok(winpane::PanelConfig {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            draggable: self.draggable != 0,
            drag_height: self.drag_height,
        })
    }
}

#[repr(C)]
pub struct WinpaneTrayConfig {
    pub version: u32,
    pub size: u32,
    pub icon_rgba: *const u8,
    pub icon_rgba_len: u32,
    pub icon_width: u32,
    pub icon_height: u32,
    pub tooltip: *const c_char,
}

impl WinpaneTrayConfig {
    unsafe fn to_rust(&self) -> Result<winpane::TrayConfig, String> {
        if self.version != WINPANE_CONFIG_VERSION {
            return Err(format!(
                "unsupported config version {} (expected {})",
                self.version, WINPANE_CONFIG_VERSION
            ));
        }
        if (self.size as usize) < std::mem::size_of::<Self>() {
            return Err(format!(
                "config size {} too small (expected at least {})",
                self.size,
                std::mem::size_of::<Self>()
            ));
        }
        require_non_null(self.icon_rgba, "icon_rgba")?;
        let icon_data =
            unsafe { std::slice::from_raw_parts(self.icon_rgba, self.icon_rgba_len as usize) };
        let tooltip = unsafe { cstr_to_string(self.tooltip)? };
        Ok(winpane::TrayConfig {
            icon_rgba: icon_data.to_vec(),
            icon_width: self.icon_width,
            icon_height: self.icon_height,
            tooltip,
        })
    }
}

// --- Element structs (value types, frozen per major version) ---

#[repr(C)]
pub struct WinpaneTextElement {
    pub text: *const c_char,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    pub color: WinpaneColor,
    pub font_family: *const c_char, // NULL for system default
    pub bold: i32,
    pub italic: i32,
    pub interactive: i32,
}

impl WinpaneTextElement {
    unsafe fn to_rust(&self) -> Result<winpane::TextElement, String> {
        let text = unsafe { cstr_to_string(self.text)? };
        let font_family = if self.font_family.is_null() {
            None
        } else {
            Some(unsafe { cstr_to_string(self.font_family)? })
        };
        Ok(winpane::TextElement {
            text,
            x: self.x,
            y: self.y,
            font_size: self.font_size,
            color: self.color.to_rust(),
            font_family,
            bold: self.bold != 0,
            italic: self.italic != 0,
            interactive: self.interactive != 0,
        })
    }
}

#[repr(C)]
pub struct WinpaneRectElement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fill: WinpaneColor,
    pub corner_radius: f32,
    pub has_border: i32,
    pub border_color: WinpaneColor,
    pub border_width: f32,
    pub interactive: i32,
}

impl WinpaneRectElement {
    fn to_rust(&self) -> winpane::RectElement {
        let border_color = if self.has_border != 0 {
            Some(self.border_color.to_rust())
        } else {
            None
        };
        winpane::RectElement {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            fill: self.fill.to_rust(),
            corner_radius: self.corner_radius,
            border_color,
            border_width: self.border_width,
            interactive: self.interactive != 0,
        }
    }
}

#[repr(C)]
pub struct WinpaneImageElement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub data: *const u8,
    pub data_len: u32,
    pub data_width: u32,
    pub data_height: u32,
    pub interactive: i32,
}

impl WinpaneImageElement {
    unsafe fn to_rust(&self) -> Result<winpane::ImageElement, String> {
        require_non_null(self.data, "image data")?;
        let data = unsafe { std::slice::from_raw_parts(self.data, self.data_len as usize) };
        Ok(winpane::ImageElement {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            data: data.to_vec(),
            data_width: self.data_width,
            data_height: self.data_height,
            interactive: self.interactive != 0,
        })
    }
}

// --- Menu item ---

#[repr(C)]
pub struct WinpaneMenuItem {
    pub id: u32,
    pub label: *const c_char,
    pub enabled: i32,
}

// --- Event ---

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinpaneEventType {
    None = 0,
    ElementClicked = 1,
    ElementHovered = 2,
    ElementLeft = 3,
    TrayClicked = 4,
    TrayMenuItemClicked = 5,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinpaneMouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

#[repr(C)]
pub struct WinpaneEvent {
    pub event_type: WinpaneEventType,
    pub surface_id: u64,
    pub key: [u8; 256], // null-terminated UTF-8
    pub mouse_button: WinpaneMouseButton,
    pub menu_item_id: u32,
}

impl WinpaneEvent {
    fn from_rust(event: &winpane::Event) -> Self {
        let mut e = WinpaneEvent {
            event_type: WinpaneEventType::None,
            surface_id: 0,
            key: [0u8; 256],
            mouse_button: WinpaneMouseButton::Left,
            menu_item_id: 0,
        };
        match event {
            winpane::Event::ElementClicked { surface_id, key } => {
                e.event_type = WinpaneEventType::ElementClicked;
                e.surface_id = surface_id.0;
                copy_key_to_buffer(key, &mut e.key);
            }
            winpane::Event::ElementHovered { surface_id, key } => {
                e.event_type = WinpaneEventType::ElementHovered;
                e.surface_id = surface_id.0;
                copy_key_to_buffer(key, &mut e.key);
            }
            winpane::Event::ElementLeft { surface_id, key } => {
                e.event_type = WinpaneEventType::ElementLeft;
                e.surface_id = surface_id.0;
                copy_key_to_buffer(key, &mut e.key);
            }
            winpane::Event::TrayClicked { button } => {
                e.event_type = WinpaneEventType::TrayClicked;
                e.mouse_button = match button {
                    winpane::MouseButton::Left => WinpaneMouseButton::Left,
                    winpane::MouseButton::Right => WinpaneMouseButton::Right,
                    winpane::MouseButton::Middle => WinpaneMouseButton::Middle,
                };
            }
            winpane::Event::TrayMenuItemClicked { id } => {
                e.event_type = WinpaneEventType::TrayMenuItemClicked;
                e.menu_item_id = *id;
            }
        }
        e
    }
}

fn copy_key_to_buffer(key: &str, buf: &mut [u8; 256]) {
    let bytes = key.as_bytes();
    let len = bytes.len().min(255); // leave room for null terminator
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
}

// ============================================================
// Opaque handle types (NOT #[repr(C)] - cbindgen generates forward declarations)
// ============================================================

/// Internal surface wrapper that unifies Hud and Panel behind one handle.
enum FfiSurface {
    Hud(winpane::Hud),
    Panel(winpane::Panel),
}

impl FfiSurface {
    fn id(&self) -> winpane::SurfaceId {
        match self {
            FfiSurface::Hud(h) => h.id(),
            FfiSurface::Panel(p) => p.id(),
        }
    }

    fn set_text(&self, key: &str, elem: winpane::TextElement) {
        match self {
            FfiSurface::Hud(h) => h.set_text(key, elem),
            FfiSurface::Panel(p) => p.set_text(key, elem),
        }
    }

    fn set_rect(&self, key: &str, elem: winpane::RectElement) {
        match self {
            FfiSurface::Hud(h) => h.set_rect(key, elem),
            FfiSurface::Panel(p) => p.set_rect(key, elem),
        }
    }

    fn set_image(&self, key: &str, elem: winpane::ImageElement) {
        match self {
            FfiSurface::Hud(h) => h.set_image(key, elem),
            FfiSurface::Panel(p) => p.set_image(key, elem),
        }
    }

    fn remove(&self, key: &str) {
        match self {
            FfiSurface::Hud(h) => h.remove(key),
            FfiSurface::Panel(p) => p.remove(key),
        }
    }

    fn show(&self) {
        match self {
            FfiSurface::Hud(h) => h.show(),
            FfiSurface::Panel(p) => p.show(),
        }
    }

    fn hide(&self) {
        match self {
            FfiSurface::Hud(h) => h.hide(),
            FfiSurface::Panel(p) => p.hide(),
        }
    }

    fn set_position(&self, x: i32, y: i32) {
        match self {
            FfiSurface::Hud(h) => h.set_position(x, y),
            FfiSurface::Panel(p) => p.set_position(x, y),
        }
    }

    fn set_size(&self, width: u32, height: u32) {
        match self {
            FfiSurface::Hud(h) => h.set_size(width, height),
            FfiSurface::Panel(p) => p.set_size(width, height),
        }
    }

    fn set_opacity(&self, opacity: f32) {
        match self {
            FfiSurface::Hud(h) => h.set_opacity(opacity),
            FfiSurface::Panel(p) => p.set_opacity(opacity),
        }
    }

    fn custom_draw(&self, ops: Vec<winpane::DrawOp>) {
        match self {
            FfiSurface::Hud(h) => h.custom_draw(ops),
            FfiSurface::Panel(p) => p.custom_draw(ops),
        }
    }
}

pub struct WinpaneContext {
    inner: winpane::Context,
}

pub struct WinpaneSurface {
    inner: FfiSurface,
    /// Active canvas for custom draw (one at a time per surface).
    canvas: Option<Box<CanvasAccumulator>>,
}

pub struct WinpaneTray {
    inner: winpane::Tray,
}

struct CanvasAccumulator {
    ops: Vec<winpane::DrawOp>,
}

pub struct WinpaneCanvas {
    ops: *mut Vec<winpane::DrawOp>,
}

// ============================================================
// Context lifecycle
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_create(out: *mut *mut WinpaneContext) -> i32 {
    ffi_try!({
        require_non_null_mut(out, "out")?;
        let ctx = winpane::Context::new().map_err(|e| e.to_string())?;
        let boxed = Box::new(WinpaneContext { inner: ctx });
        unsafe { *out = Box::into_raw(boxed) };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_destroy(ctx: *mut WinpaneContext) {
    if !ctx.is_null() {
        // Safety: ctx was created by winpane_create via Box::into_raw
        let _ = unsafe { Box::from_raw(ctx) };
    }
}

// ============================================================
// Event polling
// ============================================================

/// Polls for the next event. Returns 0 if an event was available
/// (event struct filled), 1 if no event pending, -1/-2 on error/panic.
#[no_mangle]
pub unsafe extern "C" fn winpane_poll_event(
    ctx: *mut WinpaneContext,
    event: *mut WinpaneEvent,
) -> i32 {
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        require_non_null(ctx, "ctx")?;
        require_non_null_mut(event, "event")?;
        let ctx = unsafe { &*ctx };
        match ctx.inner.poll_event() {
            Some(e) => {
                unsafe { *event = WinpaneEvent::from_rust(&e) };
                Ok(true) // event available
            }
            None => {
                unsafe { (*event).event_type = WinpaneEventType::None };
                Ok(false) // no event
            }
        }
    })) {
        Ok(Ok(true)) => 0_i32,  // event available
        Ok(Ok(false)) => 1_i32, // no event pending
        Ok(Err(e)) => {
            set_last_error(&e);
            -1_i32
        }
        Err(_) => {
            set_last_error("panic caught at FFI boundary");
            -2_i32
        }
    }
}
