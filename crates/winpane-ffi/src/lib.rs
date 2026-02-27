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

// --- PiP config (versioned) ---

#[repr(C)]
pub struct WinpanePipConfig {
    pub version: u32,
    pub size: u32,
    pub source_hwnd: isize,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl WinpanePipConfig {
    fn to_rust(&self) -> Result<winpane::PipConfig, String> {
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
        Ok(winpane::PipConfig {
            source_hwnd: self.source_hwnd,
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        })
    }
}

// --- Source rect (value type, frozen) ---

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WinpaneSourceRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WinpaneSourceRect {
    fn to_rust(&self) -> winpane::SourceRect {
        winpane::SourceRect {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }
}

// --- Backdrop ---

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinpaneBackdrop {
    None = 0,
    Mica = 1,
    Acrylic = 2,
}

impl WinpaneBackdrop {
    fn to_rust(&self) -> winpane::Backdrop {
        match self {
            WinpaneBackdrop::None => winpane::Backdrop::None,
            WinpaneBackdrop::Mica => winpane::Backdrop::Mica,
            WinpaneBackdrop::Acrylic => winpane::Backdrop::Acrylic,
        }
    }
}

// --- Anchor ---

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinpaneAnchor {
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 2,
    BottomRight = 3,
}

impl WinpaneAnchor {
    fn to_rust(&self) -> winpane::Anchor {
        match self {
            WinpaneAnchor::TopLeft => winpane::Anchor::TopLeft,
            WinpaneAnchor::TopRight => winpane::Anchor::TopRight,
            WinpaneAnchor::BottomLeft => winpane::Anchor::BottomLeft,
            WinpaneAnchor::BottomRight => winpane::Anchor::BottomRight,
        }
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
    PipSourceClosed = 6,
    AnchorTargetClosed = 7,
    DeviceRecovered = 8,
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
            winpane::Event::PipSourceClosed { surface_id } => {
                e.event_type = WinpaneEventType::PipSourceClosed;
                e.surface_id = surface_id.0;
            }
            winpane::Event::AnchorTargetClosed { surface_id } => {
                e.event_type = WinpaneEventType::AnchorTargetClosed;
                e.surface_id = surface_id.0;
            }
            winpane::Event::DeviceRecovered => {
                e.event_type = WinpaneEventType::DeviceRecovered;
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

/// Internal surface wrapper that unifies Hud, Panel, and Pip behind one handle.
enum FfiSurface {
    Hud(winpane::Hud),
    Panel(winpane::Panel),
    Pip(winpane::Pip),
}

impl FfiSurface {
    fn id(&self) -> winpane::SurfaceId {
        match self {
            FfiSurface::Hud(h) => h.id(),
            FfiSurface::Panel(p) => p.id(),
            FfiSurface::Pip(p) => p.id(),
        }
    }

    fn set_text(&self, key: &str, elem: winpane::TextElement) -> Result<(), String> {
        match self {
            FfiSurface::Hud(h) => {
                h.set_text(key, elem);
                Ok(())
            }
            FfiSurface::Panel(p) => {
                p.set_text(key, elem);
                Ok(())
            }
            FfiSurface::Pip(_) => Err("set_text is not supported on PiP surfaces".into()),
        }
    }

    fn set_rect(&self, key: &str, elem: winpane::RectElement) -> Result<(), String> {
        match self {
            FfiSurface::Hud(h) => {
                h.set_rect(key, elem);
                Ok(())
            }
            FfiSurface::Panel(p) => {
                p.set_rect(key, elem);
                Ok(())
            }
            FfiSurface::Pip(_) => Err("set_rect is not supported on PiP surfaces".into()),
        }
    }

    fn set_image(&self, key: &str, elem: winpane::ImageElement) -> Result<(), String> {
        match self {
            FfiSurface::Hud(h) => {
                h.set_image(key, elem);
                Ok(())
            }
            FfiSurface::Panel(p) => {
                p.set_image(key, elem);
                Ok(())
            }
            FfiSurface::Pip(_) => Err("set_image is not supported on PiP surfaces".into()),
        }
    }

    fn remove(&self, key: &str) -> Result<(), String> {
        match self {
            FfiSurface::Hud(h) => {
                h.remove(key);
                Ok(())
            }
            FfiSurface::Panel(p) => {
                p.remove(key);
                Ok(())
            }
            FfiSurface::Pip(_) => Err("remove is not supported on PiP surfaces".into()),
        }
    }

    fn show(&self) {
        match self {
            FfiSurface::Hud(h) => h.show(),
            FfiSurface::Panel(p) => p.show(),
            FfiSurface::Pip(p) => p.show(),
        }
    }

    fn hide(&self) {
        match self {
            FfiSurface::Hud(h) => h.hide(),
            FfiSurface::Panel(p) => p.hide(),
            FfiSurface::Pip(p) => p.hide(),
        }
    }

    fn set_position(&self, x: i32, y: i32) {
        match self {
            FfiSurface::Hud(h) => h.set_position(x, y),
            FfiSurface::Panel(p) => p.set_position(x, y),
            FfiSurface::Pip(p) => p.set_position(x, y),
        }
    }

    fn set_size(&self, width: u32, height: u32) {
        match self {
            FfiSurface::Hud(h) => h.set_size(width, height),
            FfiSurface::Panel(p) => p.set_size(width, height),
            FfiSurface::Pip(p) => p.set_size(width, height),
        }
    }

    fn set_opacity(&self, opacity: f32) {
        match self {
            FfiSurface::Hud(h) => h.set_opacity(opacity),
            FfiSurface::Panel(p) => p.set_opacity(opacity),
            FfiSurface::Pip(p) => p.set_opacity(opacity),
        }
    }

    fn custom_draw(&self, ops: Vec<winpane::DrawOp>) -> Result<(), String> {
        match self {
            FfiSurface::Hud(h) => {
                h.custom_draw(ops);
                Ok(())
            }
            FfiSurface::Panel(p) => {
                p.custom_draw(ops);
                Ok(())
            }
            FfiSurface::Pip(_) => Err("custom_draw is not supported on PiP surfaces".into()),
        }
    }

    fn anchor_to(&self, target_hwnd: isize, anchor: winpane::Anchor, offset: (i32, i32)) {
        match self {
            FfiSurface::Hud(h) => h.anchor_to(target_hwnd, anchor, offset),
            FfiSurface::Panel(p) => p.anchor_to(target_hwnd, anchor, offset),
            FfiSurface::Pip(p) => p.anchor_to(target_hwnd, anchor, offset),
        }
    }

    fn unanchor(&self) {
        match self {
            FfiSurface::Hud(h) => h.unanchor(),
            FfiSurface::Panel(p) => p.unanchor(),
            FfiSurface::Pip(p) => p.unanchor(),
        }
    }

    fn set_capture_excluded(&self, excluded: bool) {
        match self {
            FfiSurface::Hud(h) => h.set_capture_excluded(excluded),
            FfiSurface::Panel(p) => p.set_capture_excluded(excluded),
            FfiSurface::Pip(p) => p.set_capture_excluded(excluded),
        }
    }

    fn set_backdrop(&self, backdrop: winpane::Backdrop) {
        match self {
            FfiSurface::Hud(h) => h.set_backdrop(backdrop),
            FfiSurface::Panel(p) => p.set_backdrop(backdrop),
            FfiSurface::Pip(p) => p.set_backdrop(backdrop),
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

// ============================================================
// Surface creation
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_hud_create(
    ctx: *mut WinpaneContext,
    config: *const WinpaneHudConfig,
    out: *mut *mut WinpaneSurface,
) -> i32 {
    ffi_try!({
        require_non_null(ctx, "ctx")?;
        require_non_null(config, "config")?;
        require_non_null_mut(out, "out")?;
        let ctx = unsafe { &*ctx };
        let cfg = unsafe { &*config }.to_rust()?;
        let hud = ctx.inner.create_hud(cfg).map_err(|e| e.to_string())?;
        let surface = Box::new(WinpaneSurface {
            inner: FfiSurface::Hud(hud),
            canvas: None,
        });
        unsafe { *out = Box::into_raw(surface) };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_panel_create(
    ctx: *mut WinpaneContext,
    config: *const WinpanePanelConfig,
    out: *mut *mut WinpaneSurface,
) -> i32 {
    ffi_try!({
        require_non_null(ctx, "ctx")?;
        require_non_null(config, "config")?;
        require_non_null_mut(out, "out")?;
        let ctx = unsafe { &*ctx };
        let cfg = unsafe { &*config }.to_rust()?;
        let panel = ctx.inner.create_panel(cfg).map_err(|e| e.to_string())?;
        let surface = Box::new(WinpaneSurface {
            inner: FfiSurface::Panel(panel),
            canvas: None,
        });
        unsafe { *out = Box::into_raw(surface) };
        Ok(())
    })
}

// ============================================================
// Surface operations (unified for Hud and Panel)
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_destroy(surface: *mut WinpaneSurface) {
    if !surface.is_null() {
        let _ = unsafe { Box::from_raw(surface) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_id(surface: *const WinpaneSurface) -> u64 {
    if surface.is_null() {
        return 0;
    }
    unsafe { &*surface }.inner.id().0
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_text(
    surface: *mut WinpaneSurface,
    key: *const c_char,
    element: *const WinpaneTextElement,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(key, "key")?;
        require_non_null(element, "element")?;
        let surface = unsafe { &*surface };
        let key = unsafe { cstr_to_string(key)? };
        let elem = unsafe { &*element }.to_rust()?;
        surface.inner.set_text(&key, elem)?;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_rect(
    surface: *mut WinpaneSurface,
    key: *const c_char,
    element: *const WinpaneRectElement,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(key, "key")?;
        require_non_null(element, "element")?;
        let surface = unsafe { &*surface };
        let key = unsafe { cstr_to_string(key)? };
        let elem = unsafe { &*element }.to_rust();
        surface.inner.set_rect(&key, elem)?;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_image(
    surface: *mut WinpaneSurface,
    key: *const c_char,
    element: *const WinpaneImageElement,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(key, "key")?;
        require_non_null(element, "element")?;
        let surface = unsafe { &*surface };
        let key = unsafe { cstr_to_string(key)? };
        let elem = unsafe { &*element }.to_rust()?;
        surface.inner.set_image(&key, elem)?;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_remove(
    surface: *mut WinpaneSurface,
    key: *const c_char,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(key, "key")?;
        let surface = unsafe { &*surface };
        let key = unsafe { cstr_to_string(key)? };
        surface.inner.remove(&key)?;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_show(surface: *mut WinpaneSurface) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.show();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_hide(surface: *mut WinpaneSurface) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.hide();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_position(
    surface: *mut WinpaneSurface,
    x: i32,
    y: i32,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.set_position(x, y);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_size(
    surface: *mut WinpaneSurface,
    width: u32,
    height: u32,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.set_size(width, height);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_opacity(
    surface: *mut WinpaneSurface,
    opacity: f32,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.set_opacity(opacity);
        Ok(())
    })
}

// ============================================================
// Tray
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_create(
    ctx: *mut WinpaneContext,
    config: *const WinpaneTrayConfig,
    out: *mut *mut WinpaneTray,
) -> i32 {
    ffi_try!({
        require_non_null(ctx, "ctx")?;
        require_non_null(config, "config")?;
        require_non_null_mut(out, "out")?;
        let ctx = unsafe { &*ctx };
        let cfg = unsafe { &*config }.to_rust()?;
        let tray = ctx.inner.create_tray(cfg).map_err(|e| e.to_string())?;
        let boxed = Box::new(WinpaneTray { inner: tray });
        unsafe { *out = Box::into_raw(boxed) };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_destroy(tray: *mut WinpaneTray) {
    if !tray.is_null() {
        let _ = unsafe { Box::from_raw(tray) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_set_tooltip(
    tray: *mut WinpaneTray,
    tooltip: *const c_char,
) -> i32 {
    ffi_try!({
        require_non_null(tray, "tray")?;
        require_non_null(tooltip, "tooltip")?;
        let tray = unsafe { &*tray };
        let text = unsafe { cstr_to_string(tooltip)? };
        tray.inner.set_tooltip(&text);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_set_icon(
    tray: *mut WinpaneTray,
    rgba: *const u8,
    rgba_len: u32,
    width: u32,
    height: u32,
) -> i32 {
    ffi_try!({
        require_non_null(tray, "tray")?;
        require_non_null(rgba, "rgba")?;
        let tray = unsafe { &*tray };
        let data = unsafe { std::slice::from_raw_parts(rgba, rgba_len as usize) };
        tray.inner.set_icon(data.to_vec(), width, height);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_set_popup(
    tray: *mut WinpaneTray,
    panel: *const WinpaneSurface,
) -> i32 {
    ffi_try!({
        require_non_null(tray, "tray")?;
        require_non_null(panel, "panel")?;
        let tray = unsafe { &*tray };
        let surface = unsafe { &*panel };
        match &surface.inner {
            FfiSurface::Panel(p) => {
                tray.inner.set_popup(p);
                Ok(())
            }
            _ => Err("set_popup requires a panel surface".into()),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_set_menu(
    tray: *mut WinpaneTray,
    items: *const WinpaneMenuItem,
    count: u32,
) -> i32 {
    ffi_try!({
        require_non_null(tray, "tray")?;
        if count > 0 {
            require_non_null(items, "items")?;
        }
        let tray = unsafe { &*tray };
        let menu_items: Result<Vec<winpane::MenuItem>, String> = (0..count)
            .map(|i| {
                let item = unsafe { &*items.add(i as usize) };
                let label = unsafe { cstr_to_string(item.label)? };
                Ok(winpane::MenuItem {
                    id: item.id,
                    label,
                    enabled: item.enabled != 0,
                })
            })
            .collect();
        tray.inner.set_menu(menu_items?);
        Ok(())
    })
}

// ============================================================
// Custom draw (canvas)
// ============================================================

/// Begins a custom draw session on the surface. Returns a canvas handle
/// through `out`. Only one canvas can be active per surface at a time.
#[no_mangle]
pub unsafe extern "C" fn winpane_surface_begin_draw(
    surface: *mut WinpaneSurface,
    out: *mut *mut WinpaneCanvas,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null_mut(out, "out")?;
        let surface = unsafe { &mut *surface };
        if surface.canvas.is_some() {
            return Err("a canvas is already active on this surface; call end_draw first".into());
        }
        let mut acc = Box::new(CanvasAccumulator { ops: Vec::new() });
        let ops_ptr: *mut Vec<winpane::DrawOp> = &mut acc.ops;
        surface.canvas = Some(acc);
        let canvas = Box::new(WinpaneCanvas { ops: ops_ptr });
        unsafe { *out = Box::into_raw(canvas) };
        Ok(())
    })
}

/// Ends the custom draw session, flushing all accumulated draw ops to the surface.
/// The canvas handle is invalid after this call - do not use it.
#[no_mangle]
pub unsafe extern "C" fn winpane_surface_end_draw(surface: *mut WinpaneSurface) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        let surface = unsafe { &mut *surface };
        let acc = surface
            .canvas
            .take()
            .ok_or_else(|| "no active canvas; call begin_draw first".to_string())?;
        surface.inner.custom_draw(acc.ops)?;
        Ok(())
    })
}

// --- Canvas drawing functions ---
// Each pushes a DrawOp to the accumulator.

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_clear(
    canvas: *mut WinpaneCanvas,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::Clear(color.to_rust()));
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_fill_rect(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::FillRect {
            x,
            y,
            width: w,
            height: h,
            color: color.to_rust(),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_stroke_rect(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: WinpaneColor,
    width: f32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::StrokeRect {
            x,
            y,
            width: w,
            height: h,
            color: color.to_rust(),
            stroke_width: width,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_draw_text(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    text: *const c_char,
    font_size: f32,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let text_str = unsafe { cstr_to_string(text)? };
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::DrawText {
            x,
            y,
            text: text_str,
            font_size,
            color: color.to_rust(),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_draw_line(
    canvas: *mut WinpaneCanvas,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    color: WinpaneColor,
    width: f32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::DrawLine {
            x1,
            y1,
            x2,
            y2,
            color: color.to_rust(),
            stroke_width: width,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_fill_ellipse(
    canvas: *mut WinpaneCanvas,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::FillEllipse {
            cx,
            cy,
            rx,
            ry,
            color: color.to_rust(),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_stroke_ellipse(
    canvas: *mut WinpaneCanvas,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    color: WinpaneColor,
    width: f32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::StrokeEllipse {
            cx,
            cy,
            rx,
            ry,
            color: color.to_rust(),
            stroke_width: width,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_draw_image(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rgba: *const u8,
    rgba_len: u32,
    img_w: u32,
    img_h: u32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        require_non_null(rgba, "rgba")?;
        let data = unsafe { std::slice::from_raw_parts(rgba, rgba_len as usize) };
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::DrawImage {
            x,
            y,
            width: w,
            height: h,
            rgba: data.to_vec(),
            img_width: img_w,
            img_height: img_h,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_fill_rounded_rect(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::FillRoundedRect {
            x,
            y,
            width: w,
            height: h,
            radius,
            color: color.to_rust(),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_stroke_rounded_rect(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: WinpaneColor,
    width: f32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::StrokeRoundedRect {
            x,
            y,
            width: w,
            height: h,
            radius,
            color: color.to_rust(),
            stroke_width: width,
        });
        Ok(())
    })
}

// ============================================================
// PiP creation
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_pip_create(
    ctx: *mut WinpaneContext,
    config: *const WinpanePipConfig,
    out: *mut *mut WinpaneSurface,
) -> i32 {
    ffi_try!({
        require_non_null(ctx, "ctx")?;
        require_non_null(config, "config")?;
        require_non_null_mut(out, "out")?;
        let ctx = unsafe { &*ctx };
        let cfg = unsafe { &*config }.to_rust()?;
        let pip = ctx.inner.create_pip(cfg).map_err(|e| e.to_string())?;
        let surface = Box::new(WinpaneSurface {
            inner: FfiSurface::Pip(pip),
            canvas: None,
        });
        unsafe { *out = Box::into_raw(surface) };
        Ok(())
    })
}

// ============================================================
// PiP source region
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_source_region(
    surface: *mut WinpaneSurface,
    rect: *const WinpaneSourceRect,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(rect, "rect")?;
        let surface = unsafe { &*surface };
        match &surface.inner {
            FfiSurface::Pip(p) => {
                let r = unsafe { &*rect }.to_rust();
                p.set_source_region(r);
                Ok(())
            }
            _ => Err("set_source_region is only valid on PiP surfaces".into()),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_clear_source_region(surface: *mut WinpaneSurface) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        let surface = unsafe { &*surface };
        match &surface.inner {
            FfiSurface::Pip(p) => {
                p.clear_source_region();
                Ok(())
            }
            _ => Err("clear_source_region is only valid on PiP surfaces".into()),
        }
    })
}

// ============================================================
// Anchoring
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_anchor_to(
    surface: *mut WinpaneSurface,
    target_hwnd: isize,
    anchor: WinpaneAnchor,
    offset_x: i32,
    offset_y: i32,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        let surface = unsafe { &*surface };
        surface
            .inner
            .anchor_to(target_hwnd, anchor.to_rust(), (offset_x, offset_y));
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_unanchor(surface: *mut WinpaneSurface) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        let surface = unsafe { &*surface };
        surface.inner.unanchor();
        Ok(())
    })
}

// ============================================================
// Capture exclusion
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_capture_excluded(
    surface: *mut WinpaneSurface,
    excluded: i32,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        let surface = unsafe { &*surface };
        surface.inner.set_capture_excluded(excluded != 0);
        Ok(())
    })
}

// ============================================================
// Backdrop
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_backdrop(
    surface: *mut WinpaneSurface,
    backdrop: WinpaneBackdrop,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        let surface = unsafe { &*surface };
        surface.inner.set_backdrop(backdrop.to_rust());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn winpane_backdrop_supported() -> i32 {
    if winpane::backdrop_supported() {
        1
    } else {
        0
    }
}
