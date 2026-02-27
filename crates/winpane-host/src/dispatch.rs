use std::collections::HashMap;

use serde_json::Value;
use winpane::{
    Anchor, Backdrop, Context, Event, Hud, HudConfig, ImageElement, MenuItem, MouseButton, Panel,
    PanelConfig, Pip, PipConfig, RectElement, SourceRect, SurfaceId, TextElement, Tray, TrayConfig,
};

use crate::protocol::{INTERNAL_ERROR, INVALID_PARAMS, METHOD_NOT_FOUND};
use crate::util::{extract_optional_color, load_image_rgba, parse_color};

// ---------------------------------------------------------------------------
// SurfaceHandle
// ---------------------------------------------------------------------------

enum SurfaceHandle {
    Hud(Hud),
    Panel(Panel),
    Pip(Pip),
}

impl SurfaceHandle {
    fn id(&self) -> SurfaceId {
        match self {
            SurfaceHandle::Hud(h) => h.id(),
            SurfaceHandle::Panel(p) => p.id(),
            SurfaceHandle::Pip(p) => p.id(),
        }
    }

    fn set_text(&self, key: &str, element: TextElement) -> Result<(), (i32, String)> {
        match self {
            SurfaceHandle::Hud(h) => {
                h.set_text(key, element);
                Ok(())
            }
            SurfaceHandle::Panel(p) => {
                p.set_text(key, element);
                Ok(())
            }
            SurfaceHandle::Pip(_) => Err((
                INVALID_PARAMS,
                "set_text not supported on PiP surfaces".into(),
            )),
        }
    }

    fn set_rect(&self, key: &str, element: RectElement) -> Result<(), (i32, String)> {
        match self {
            SurfaceHandle::Hud(h) => {
                h.set_rect(key, element);
                Ok(())
            }
            SurfaceHandle::Panel(p) => {
                p.set_rect(key, element);
                Ok(())
            }
            SurfaceHandle::Pip(_) => Err((
                INVALID_PARAMS,
                "set_rect not supported on PiP surfaces".into(),
            )),
        }
    }

    fn set_image(&self, key: &str, element: ImageElement) -> Result<(), (i32, String)> {
        match self {
            SurfaceHandle::Hud(h) => {
                h.set_image(key, element);
                Ok(())
            }
            SurfaceHandle::Panel(p) => {
                p.set_image(key, element);
                Ok(())
            }
            SurfaceHandle::Pip(_) => Err((
                INVALID_PARAMS,
                "set_image not supported on PiP surfaces".into(),
            )),
        }
    }

    fn remove(&self, key: &str) -> Result<(), (i32, String)> {
        match self {
            SurfaceHandle::Hud(h) => {
                h.remove(key);
                Ok(())
            }
            SurfaceHandle::Panel(p) => {
                p.remove(key);
                Ok(())
            }
            SurfaceHandle::Pip(_) => Err((
                INVALID_PARAMS,
                "remove_element not supported on PiP surfaces".into(),
            )),
        }
    }

    fn show(&self) {
        match self {
            SurfaceHandle::Hud(h) => h.show(),
            SurfaceHandle::Panel(p) => p.show(),
            SurfaceHandle::Pip(p) => p.show(),
        }
    }

    fn hide(&self) {
        match self {
            SurfaceHandle::Hud(h) => h.hide(),
            SurfaceHandle::Panel(p) => p.hide(),
            SurfaceHandle::Pip(p) => p.hide(),
        }
    }

    fn set_position(&self, x: i32, y: i32) {
        match self {
            SurfaceHandle::Hud(h) => h.set_position(x, y),
            SurfaceHandle::Panel(p) => p.set_position(x, y),
            SurfaceHandle::Pip(p) => p.set_position(x, y),
        }
    }

    fn set_size(&self, width: u32, height: u32) {
        match self {
            SurfaceHandle::Hud(h) => h.set_size(width, height),
            SurfaceHandle::Panel(p) => p.set_size(width, height),
            SurfaceHandle::Pip(p) => p.set_size(width, height),
        }
    }

    fn set_opacity(&self, opacity: f32) {
        match self {
            SurfaceHandle::Hud(h) => h.set_opacity(opacity),
            SurfaceHandle::Panel(p) => p.set_opacity(opacity),
            SurfaceHandle::Pip(p) => p.set_opacity(opacity),
        }
    }

    fn anchor_to(&self, target_hwnd: isize, anchor: Anchor, offset: (i32, i32)) {
        match self {
            SurfaceHandle::Hud(h) => h.anchor_to(target_hwnd, anchor, offset),
            SurfaceHandle::Panel(p) => p.anchor_to(target_hwnd, anchor, offset),
            SurfaceHandle::Pip(p) => p.anchor_to(target_hwnd, anchor, offset),
        }
    }

    fn unanchor(&self) {
        match self {
            SurfaceHandle::Hud(h) => h.unanchor(),
            SurfaceHandle::Panel(p) => p.unanchor(),
            SurfaceHandle::Pip(p) => p.unanchor(),
        }
    }

    fn set_capture_excluded(&self, excluded: bool) {
        match self {
            SurfaceHandle::Hud(h) => h.set_capture_excluded(excluded),
            SurfaceHandle::Panel(p) => p.set_capture_excluded(excluded),
            SurfaceHandle::Pip(p) => p.set_capture_excluded(excluded),
        }
    }

    fn set_backdrop(&self, backdrop: Backdrop) {
        match self {
            SurfaceHandle::Hud(h) => h.set_backdrop(backdrop),
            SurfaceHandle::Panel(p) => p.set_backdrop(backdrop),
            SurfaceHandle::Pip(p) => p.set_backdrop(backdrop),
        }
    }

    fn fade_in(&self, duration_ms: u32) {
        match self {
            SurfaceHandle::Hud(h) => h.fade_in(duration_ms),
            SurfaceHandle::Panel(p) => p.fade_in(duration_ms),
            SurfaceHandle::Pip(p) => p.fade_in(duration_ms),
        }
    }

    fn fade_out(&self, duration_ms: u32) {
        match self {
            SurfaceHandle::Hud(h) => h.fade_out(duration_ms),
            SurfaceHandle::Panel(p) => p.fade_out(duration_ms),
            SurfaceHandle::Pip(p) => p.fade_out(duration_ms),
        }
    }

    fn set_source_region(&self, rect: SourceRect) -> Result<(), (i32, String)> {
        match self {
            SurfaceHandle::Pip(p) => {
                p.set_source_region(rect);
                Ok(())
            }
            _ => Err((
                INVALID_PARAMS,
                "set_source_region only supported on PiP surfaces".into(),
            )),
        }
    }

    fn clear_source_region(&self) -> Result<(), (i32, String)> {
        match self {
            SurfaceHandle::Pip(p) => {
                p.clear_source_region();
                Ok(())
            }
            _ => Err((
                INVALID_PARAMS,
                "clear_source_region only supported on PiP surfaces".into(),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Param extraction helpers
// ---------------------------------------------------------------------------

fn get_i32(params: &Value, key: &str) -> Result<i32, (i32, String)> {
    params
        .get(key)
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| {
            (
                INVALID_PARAMS,
                format!("missing or invalid {key} (expected integer)"),
            )
        })
}

fn get_u32(params: &Value, key: &str) -> Result<u32, (i32, String)> {
    params
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .ok_or_else(|| {
            (
                INVALID_PARAMS,
                format!("missing or invalid {key} (expected unsigned integer)"),
            )
        })
}

fn get_f32(params: &Value, key: &str) -> Result<f32, (i32, String)> {
    params
        .get(key)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .ok_or_else(|| {
            (
                INVALID_PARAMS,
                format!("missing or invalid {key} (expected number)"),
            )
        })
}

fn get_str<'a>(params: &'a Value, key: &str) -> Result<&'a str, (i32, String)> {
    params.get(key).and_then(|v| v.as_str()).ok_or_else(|| {
        (
            INVALID_PARAMS,
            format!("missing or invalid {key} (expected string)"),
        )
    })
}

fn get_bool(params: &Value, key: &str) -> Result<bool, (i32, String)> {
    params.get(key).and_then(|v| v.as_bool()).ok_or_else(|| {
        (
            INVALID_PARAMS,
            format!("missing or invalid {key} (expected boolean)"),
        )
    })
}

fn opt_i32(params: &Value, key: &str) -> Option<i32> {
    params.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

fn opt_u32(params: &Value, key: &str) -> Option<u32> {
    params.get(key).and_then(|v| v.as_u64()).map(|v| v as u32)
}

fn opt_f32(params: &Value, key: &str) -> Option<f32> {
    params.get(key).and_then(|v| v.as_f64()).map(|v| v as f32)
}

fn opt_str<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key).and_then(|v| v.as_str())
}

fn opt_bool(params: &Value, key: &str) -> Option<bool> {
    params.get(key).and_then(|v| v.as_bool())
}

fn parse_anchor(s: &str) -> Result<Anchor, (i32, String)> {
    match s {
        "top_left" => Ok(Anchor::TopLeft),
        "top_right" => Ok(Anchor::TopRight),
        "bottom_left" => Ok(Anchor::BottomLeft),
        "bottom_right" => Ok(Anchor::BottomRight),
        _ => Err((
            INVALID_PARAMS,
            format!(
                "invalid anchor: {s} (expected top_left, top_right, bottom_left, bottom_right)"
            ),
        )),
    }
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

pub struct Dispatcher {
    ctx: Context,
    surfaces: HashMap<String, SurfaceHandle>,
    trays: HashMap<String, Tray>,
    surface_id_to_string: HashMap<u64, String>,
    next_surface: u64,
    next_tray: u64,
}

impl Dispatcher {
    pub fn new() -> Result<Self, String> {
        let ctx = Context::new().map_err(|e| format!("failed to create context: {e}"))?;
        Ok(Self {
            ctx,
            surfaces: HashMap::new(),
            trays: HashMap::new(),
            surface_id_to_string: HashMap::new(),
            next_surface: 1,
            next_tray: 1,
        })
    }

    fn next_surface_id(&mut self) -> String {
        let id = format!("s{}", self.next_surface);
        self.next_surface += 1;
        id
    }

    fn next_tray_id(&mut self) -> String {
        let id = format!("t{}", self.next_tray);
        self.next_tray += 1;
        id
    }

    pub fn poll_event(&self) -> Option<Event> {
        self.ctx.poll_event()
    }

    pub fn dispatch(&mut self, method: &str, params: &Value) -> Result<Value, (i32, String)> {
        match method {
            // Surface creation
            "create_hud" => self.create_hud(params),
            "create_panel" => self.create_panel(params),
            "create_pip" => self.create_pip(params),
            "create_tray" => self.create_tray(params),

            // Element operations
            "set_text" => self.set_text(params),
            "set_rect" => self.set_rect(params),
            "set_image" => self.set_image(params),
            "remove_element" => self.remove_element(params),

            // Surface control
            "show" => self.show(params),
            "hide" => self.hide(params),
            "set_position" => self.set_position(params),
            "set_size" => self.set_size(params),
            "set_opacity" => self.set_opacity(params),
            "set_capture_excluded" => self.set_capture_excluded(params),

            // Anchoring
            "anchor_to" => self.anchor_to(params),
            "unanchor" => self.unanchor(params),

            // PiP-specific
            "set_source_region" => self.set_source_region(params),
            "clear_source_region" => self.clear_source_region(params),

            // Tray-specific
            "set_tooltip" => self.set_tooltip(params),
            "set_tray_icon" => self.set_tray_icon(params),
            "set_popup" => self.set_popup(params),
            "set_menu" => self.set_menu(params),

            // Fade animations
            "fade_in" => self.fade_in(params),
            "fade_out" => self.fade_out(params),

            // Backdrop
            "set_backdrop" => self.set_backdrop(params),
            "backdrop_supported" => {
                Ok(serde_json::json!({ "supported": winpane::backdrop_supported() }))
            }

            // Lifecycle
            "destroy" => self.destroy(params),

            _ => Err((METHOD_NOT_FOUND, format!("unknown method: {method}"))),
        }
    }

    // -- Surface creation ---------------------------------------------------

    fn create_hud(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let config = HudConfig {
            x: get_i32(params, "x")?,
            y: get_i32(params, "y")?,
            width: get_u32(params, "width")?,
            height: get_u32(params, "height")?,
        };
        let hud = self
            .ctx
            .create_hud(config)
            .map_err(|e| (INTERNAL_ERROR, format!("create_hud failed: {e}")))?;
        let sid = self.next_surface_id();
        self.surface_id_to_string.insert(hud.id().0, sid.clone());
        self.surfaces.insert(sid.clone(), SurfaceHandle::Hud(hud));
        Ok(serde_json::json!({ "surface_id": sid }))
    }

    fn create_panel(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let config = PanelConfig {
            x: get_i32(params, "x")?,
            y: get_i32(params, "y")?,
            width: get_u32(params, "width")?,
            height: get_u32(params, "height")?,
            draggable: opt_bool(params, "draggable").unwrap_or(false),
            drag_height: opt_u32(params, "drag_height").unwrap_or(0),
        };
        let panel = self
            .ctx
            .create_panel(config)
            .map_err(|e| (INTERNAL_ERROR, format!("create_panel failed: {e}")))?;
        let sid = self.next_surface_id();
        self.surface_id_to_string.insert(panel.id().0, sid.clone());
        self.surfaces
            .insert(sid.clone(), SurfaceHandle::Panel(panel));
        Ok(serde_json::json!({ "surface_id": sid }))
    }

    fn create_pip(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let source_hwnd = params
            .get("source_hwnd")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                (
                    INVALID_PARAMS,
                    "missing or invalid source_hwnd (expected integer)".into(),
                )
            })? as isize;
        let config = PipConfig {
            source_hwnd,
            x: get_i32(params, "x")?,
            y: get_i32(params, "y")?,
            width: get_u32(params, "width")?,
            height: get_u32(params, "height")?,
        };
        let pip = self
            .ctx
            .create_pip(config)
            .map_err(|e| (INTERNAL_ERROR, format!("create_pip failed: {e}")))?;
        let sid = self.next_surface_id();
        self.surface_id_to_string.insert(pip.id().0, sid.clone());
        self.surfaces.insert(sid.clone(), SurfaceHandle::Pip(pip));
        Ok(serde_json::json!({ "surface_id": sid }))
    }

    fn create_tray(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let tooltip = get_str(params, "tooltip")?.to_string();
        let (icon_rgba, icon_width, icon_height) = match opt_str(params, "icon_path") {
            Some(path) => load_image_rgba(path, false).map_err(|e| (INVALID_PARAMS, e))?,
            None => {
                // Default 16x16 white icon
                let size = 16u32;
                let pixels = vec![255u8; (size * size * 4) as usize];
                (pixels, size, size)
            }
        };
        let config = TrayConfig {
            icon_rgba,
            icon_width,
            icon_height,
            tooltip,
        };
        let tray = self
            .ctx
            .create_tray(config)
            .map_err(|e| (INTERNAL_ERROR, format!("create_tray failed: {e}")))?;
        let tid = self.next_tray_id();
        self.trays.insert(tid.clone(), tray);
        Ok(serde_json::json!({ "surface_id": tid }))
    }

    // -- Element operations -------------------------------------------------

    fn set_text(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let key = get_str(params, "key")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;

        let color = extract_optional_color(params, "color")
            .map_err(|e| (INVALID_PARAMS, e))?
            .unwrap_or(winpane::Color::WHITE);

        let element = TextElement {
            text: get_str(params, "text")?.to_string(),
            x: get_f32(params, "x")?,
            y: get_f32(params, "y")?,
            font_size: get_f32(params, "font_size")?,
            color,
            font_family: opt_str(params, "font_family").map(String::from),
            bold: opt_bool(params, "bold").unwrap_or(false),
            italic: opt_bool(params, "italic").unwrap_or(false),
            interactive: opt_bool(params, "interactive").unwrap_or(false),
        };

        surface.set_text(key, element)?;
        Ok(serde_json::json!({}))
    }

    fn set_rect(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let key = get_str(params, "key")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;

        let fill = extract_optional_color(params, "fill")
            .map_err(|e| (INVALID_PARAMS, e))?
            .unwrap_or(winpane::Color::WHITE);

        let border_color =
            extract_optional_color(params, "border_color").map_err(|e| (INVALID_PARAMS, e))?;

        let element = RectElement {
            x: get_f32(params, "x")?,
            y: get_f32(params, "y")?,
            width: get_f32(params, "width")?,
            height: get_f32(params, "height")?,
            fill,
            corner_radius: opt_f32(params, "corner_radius").unwrap_or(0.0),
            border_color,
            border_width: opt_f32(params, "border_width").unwrap_or(0.0),
            interactive: opt_bool(params, "interactive").unwrap_or(false),
        };

        surface.set_rect(key, element)?;
        Ok(serde_json::json!({}))
    }

    fn set_image(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let key = get_str(params, "key")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;

        let path = get_str(params, "path")?;
        let (data, data_width, data_height) =
            load_image_rgba(path, true).map_err(|e| (INVALID_PARAMS, e))?;

        let element = ImageElement {
            x: get_f32(params, "x")?,
            y: get_f32(params, "y")?,
            width: get_f32(params, "width")?,
            height: get_f32(params, "height")?,
            data,
            data_width,
            data_height,
            interactive: opt_bool(params, "interactive").unwrap_or(false),
        };

        surface.set_image(key, element)?;
        Ok(serde_json::json!({}))
    }

    fn remove_element(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let key = get_str(params, "key")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.remove(key)?;
        Ok(serde_json::json!({}))
    }

    // -- Surface control ----------------------------------------------------

    fn show(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.show();
        Ok(serde_json::json!({}))
    }

    fn hide(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.hide();
        Ok(serde_json::json!({}))
    }

    fn set_position(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.set_position(get_i32(params, "x")?, get_i32(params, "y")?);
        Ok(serde_json::json!({}))
    }

    fn set_size(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.set_size(get_u32(params, "width")?, get_u32(params, "height")?);
        Ok(serde_json::json!({}))
    }

    fn set_opacity(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.set_opacity(get_f32(params, "opacity")?);
        Ok(serde_json::json!({}))
    }

    fn set_capture_excluded(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.set_capture_excluded(get_bool(params, "excluded")?);
        Ok(serde_json::json!({}))
    }

    // -- Fade animations ----------------------------------------------------

    fn fade_in(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        let duration_ms = get_u32(params, "duration_ms")?;
        surface.fade_in(duration_ms);
        Ok(serde_json::json!({}))
    }

    fn fade_out(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        let duration_ms = get_u32(params, "duration_ms")?;
        surface.fade_out(duration_ms);
        Ok(serde_json::json!({}))
    }

    // -- Backdrop -----------------------------------------------------------

    fn set_backdrop(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        let backdrop_str = get_str(params, "backdrop")?;
        let backdrop = match backdrop_str {
            "none" => Backdrop::None,
            "mica" => Backdrop::Mica,
            "acrylic" => Backdrop::Acrylic,
            _ => {
                return Err((
                    INVALID_PARAMS,
                    format!("invalid backdrop: {backdrop_str} (expected none, mica, acrylic)"),
                ))
            }
        };
        surface.set_backdrop(backdrop);
        Ok(serde_json::json!({}))
    }

    // -- Anchoring ----------------------------------------------------------

    fn anchor_to(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;

        let target_hwnd = params
            .get("target_hwnd")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                (
                    INVALID_PARAMS,
                    "missing or invalid target_hwnd (expected integer)".into(),
                )
            })? as isize;

        let anchor = parse_anchor(get_str(params, "anchor")?)?;
        let offset_x = opt_i32(params, "offset_x").unwrap_or(0);
        let offset_y = opt_i32(params, "offset_y").unwrap_or(0);

        surface.anchor_to(target_hwnd, anchor, (offset_x, offset_y));
        Ok(serde_json::json!({}))
    }

    fn unanchor(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.unanchor();
        Ok(serde_json::json!({}))
    }

    // -- PiP-specific -------------------------------------------------------

    fn set_source_region(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        let rect = SourceRect {
            x: get_i32(params, "x")?,
            y: get_i32(params, "y")?,
            width: get_i32(params, "width")?,
            height: get_i32(params, "height")?,
        };
        surface.set_source_region(rect)?;
        Ok(serde_json::json!({}))
    }

    fn clear_source_region(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let surface = self
            .surfaces
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        surface.clear_source_region()?;
        Ok(serde_json::json!({}))
    }

    // -- Tray-specific ------------------------------------------------------

    fn set_tooltip(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let tray = self
            .trays
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        tray.set_tooltip(get_str(params, "tooltip")?);
        Ok(serde_json::json!({}))
    }

    fn set_tray_icon(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let tray = self
            .trays
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;
        let path = get_str(params, "icon_path")?;
        let (rgba, width, height) =
            load_image_rgba(path, false).map_err(|e| (INVALID_PARAMS, e))?;
        tray.set_icon(rgba, width, height);
        Ok(serde_json::json!({}))
    }

    fn set_popup(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?.to_string();
        let panel_surface_id = get_str(params, "panel_surface_id")?.to_string();

        let panel = match self.surfaces.get(&panel_surface_id) {
            Some(SurfaceHandle::Panel(p)) => p,
            Some(_) => return Err((INVALID_PARAMS, format!("{panel_surface_id} is not a Panel"))),
            None => {
                return Err((
                    INVALID_PARAMS,
                    format!("unknown surface_id: {panel_surface_id}"),
                ))
            }
        };

        // Borrow panel ref before borrowing trays to avoid double-borrow issues
        let tray = self
            .trays
            .get(&surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;

        tray.set_popup(panel);
        Ok(serde_json::json!({}))
    }

    fn set_menu(&self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?;
        let tray = self
            .trays
            .get(surface_id)
            .ok_or_else(|| (INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))?;

        let items_val = params
            .get("items")
            .ok_or_else(|| (INVALID_PARAMS, "missing items (expected array)".into()))?;
        let items_arr = items_val
            .as_array()
            .ok_or_else(|| (INVALID_PARAMS, "items must be an array".into()))?;

        let mut items = Vec::with_capacity(items_arr.len());
        for item in items_arr {
            items.push(MenuItem {
                id: get_u32(item, "id")?,
                label: get_str(item, "label")?.to_string(),
                enabled: opt_bool(item, "enabled").unwrap_or(true),
            });
        }

        tray.set_menu(items);
        Ok(serde_json::json!({}))
    }

    // -- Lifecycle ----------------------------------------------------------

    fn destroy(&mut self, params: &Value) -> Result<Value, (i32, String)> {
        let surface_id = get_str(params, "surface_id")?.to_string();

        // Try surfaces first, then trays
        if let Some(handle) = self.surfaces.remove(&surface_id) {
            self.surface_id_to_string.remove(&handle.id().0);
            // Drop handles cleanup via Rust Drop impl
            drop(handle);
            return Ok(serde_json::json!({}));
        }

        if self.trays.remove(&surface_id).is_some() {
            return Ok(serde_json::json!({}));
        }

        Err((INVALID_PARAMS, format!("unknown surface_id: {surface_id}")))
    }
}

// ---------------------------------------------------------------------------
// Event serialization
// ---------------------------------------------------------------------------

pub fn event_to_json(event: &Event, id_map: &HashMap<u64, String>) -> Value {
    match event {
        Event::ElementClicked { surface_id, key } => {
            let sid = id_map
                .get(&surface_id.0)
                .cloned()
                .unwrap_or_else(|| format!("s?{}", surface_id.0));
            serde_json::json!({
                "type": "element_clicked",
                "surface_id": sid,
                "key": key,
            })
        }
        Event::ElementHovered { surface_id, key } => {
            let sid = id_map
                .get(&surface_id.0)
                .cloned()
                .unwrap_or_else(|| format!("s?{}", surface_id.0));
            serde_json::json!({
                "type": "element_hovered",
                "surface_id": sid,
                "key": key,
            })
        }
        Event::ElementLeft { surface_id, key } => {
            let sid = id_map
                .get(&surface_id.0)
                .cloned()
                .unwrap_or_else(|| format!("s?{}", surface_id.0));
            serde_json::json!({
                "type": "element_left",
                "surface_id": sid,
                "key": key,
            })
        }
        Event::TrayClicked { button } => {
            let button_str = match button {
                MouseButton::Left => "left",
                MouseButton::Right => "right",
                MouseButton::Middle => "middle",
            };
            serde_json::json!({
                "type": "tray_clicked",
                "button": button_str,
            })
        }
        Event::TrayMenuItemClicked { id } => {
            serde_json::json!({
                "type": "tray_menu_item_clicked",
                "item_id": id,
            })
        }
        Event::PipSourceClosed { surface_id } => {
            let sid = id_map
                .get(&surface_id.0)
                .cloned()
                .unwrap_or_else(|| format!("s?{}", surface_id.0));
            serde_json::json!({
                "type": "pip_source_closed",
                "surface_id": sid,
            })
        }
        Event::AnchorTargetClosed { surface_id } => {
            let sid = id_map
                .get(&surface_id.0)
                .cloned()
                .unwrap_or_else(|| format!("s?{}", surface_id.0));
            serde_json::json!({
                "type": "anchor_target_closed",
                "surface_id": sid,
            })
        }
        Event::DeviceRecovered => {
            serde_json::json!({"type": "device_recovered"})
        }
    }
}

/// Returns a reference to the dispatcher's surface ID reverse map.
impl Dispatcher {
    pub fn surface_id_map(&self) -> &HashMap<u64, String> {
        &self.surface_id_to_string
    }
}
