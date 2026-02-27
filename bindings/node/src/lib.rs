#![deny(clippy::all)]

use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use winpane::{
    Anchor, Backdrop, Color, HudConfig, ImageElement, MenuItem, MouseButton, PanelConfig,
    PipConfig, RectElement, SourceRect, SurfaceId, TextElement, TrayConfig,
};

// ---------------------------------------------------------------------------
// Utility functions (duplicated from host per proposal decision)
// ---------------------------------------------------------------------------

fn parse_color(s: &str) -> Result<Color> {
    let hex = s.strip_prefix('#').unwrap_or(s);

    let parse_byte = |h: &str| -> Result<u8> {
        u8::from_str_radix(h, 16)
            .map_err(|_| Error::new(Status::InvalidArg, format!("invalid hex color: {s}")))
    };

    match hex.len() {
        3 => {
            let r = parse_byte(&hex[0..1])?;
            let g = parse_byte(&hex[1..2])?;
            let b = parse_byte(&hex[2..3])?;
            Ok(Color::rgba(r << 4 | r, g << 4 | g, b << 4 | b, 255))
        }
        6 => {
            let r = parse_byte(&hex[0..2])?;
            let g = parse_byte(&hex[2..4])?;
            let b = parse_byte(&hex[4..6])?;
            Ok(Color::rgba(r, g, b, 255))
        }
        8 => {
            let r = parse_byte(&hex[0..2])?;
            let g = parse_byte(&hex[2..4])?;
            let b = parse_byte(&hex[4..6])?;
            let a = parse_byte(&hex[6..8])?;
            Ok(Color::rgba(r, g, b, a))
        }
        _ => Err(Error::new(
            Status::InvalidArg,
            format!("invalid hex color length: {s}"),
        )),
    }
}

fn color_or_default(s: Option<String>, default: Color) -> Result<Color> {
    match s {
        Some(ref v) => parse_color(v),
        None => Ok(default),
    }
}

fn load_image_rgba(path: &str, premultiply: bool) -> Result<(Vec<u8>, u32, u32)> {
    let img = image::open(path).map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("failed to load image {path}: {e}"),
        )
    })?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut pixels = rgba.into_raw();

    if premultiply {
        for chunk in pixels.chunks_exact_mut(4) {
            let a = chunk[3] as u16;
            chunk[0] = ((chunk[0] as u16 * a) / 255) as u8;
            chunk[1] = ((chunk[1] as u16 * a) / 255) as u8;
            chunk[2] = ((chunk[2] as u16 * a) / 255) as u8;
        }
    }

    Ok((pixels, width, height))
}

fn parse_anchor(s: &str) -> Result<Anchor> {
    match s {
        "top_left" => Ok(Anchor::TopLeft),
        "top_right" => Ok(Anchor::TopRight),
        "bottom_left" => Ok(Anchor::BottomLeft),
        "bottom_right" => Ok(Anchor::BottomRight),
        _ => Err(Error::new(
            Status::InvalidArg,
            format!(
                "invalid anchor: {s} (expected top_left, top_right, bottom_left, bottom_right)"
            ),
        )),
    }
}

// ---------------------------------------------------------------------------
// SurfaceHandle
// ---------------------------------------------------------------------------

enum SurfaceHandle {
    Hud(winpane::Hud),
    Panel(winpane::Panel),
    Pip(winpane::Pip),
}

impl SurfaceHandle {
    fn id(&self) -> SurfaceId {
        match self {
            SurfaceHandle::Hud(h) => h.id(),
            SurfaceHandle::Panel(p) => p.id(),
            SurfaceHandle::Pip(p) => p.id(),
        }
    }

    fn set_text(&self, key: &str, element: TextElement) -> Result<()> {
        match self {
            SurfaceHandle::Hud(h) => {
                h.set_text(key, element);
                Ok(())
            }
            SurfaceHandle::Panel(p) => {
                p.set_text(key, element);
                Ok(())
            }
            SurfaceHandle::Pip(_) => Err(Error::new(
                Status::InvalidArg,
                "set_text not supported on PiP surfaces",
            )),
        }
    }

    fn set_rect(&self, key: &str, element: RectElement) -> Result<()> {
        match self {
            SurfaceHandle::Hud(h) => {
                h.set_rect(key, element);
                Ok(())
            }
            SurfaceHandle::Panel(p) => {
                p.set_rect(key, element);
                Ok(())
            }
            SurfaceHandle::Pip(_) => Err(Error::new(
                Status::InvalidArg,
                "set_rect not supported on PiP surfaces",
            )),
        }
    }

    fn set_image(&self, key: &str, element: ImageElement) -> Result<()> {
        match self {
            SurfaceHandle::Hud(h) => {
                h.set_image(key, element);
                Ok(())
            }
            SurfaceHandle::Panel(p) => {
                p.set_image(key, element);
                Ok(())
            }
            SurfaceHandle::Pip(_) => Err(Error::new(
                Status::InvalidArg,
                "set_image not supported on PiP surfaces",
            )),
        }
    }

    fn remove(&self, key: &str) -> Result<()> {
        match self {
            SurfaceHandle::Hud(h) => {
                h.remove(key);
                Ok(())
            }
            SurfaceHandle::Panel(p) => {
                p.remove(key);
                Ok(())
            }
            SurfaceHandle::Pip(_) => Err(Error::new(
                Status::InvalidArg,
                "remove_element not supported on PiP surfaces",
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

    fn set_source_region(&self, rect: SourceRect) -> Result<()> {
        match self {
            SurfaceHandle::Pip(p) => {
                p.set_source_region(rect);
                Ok(())
            }
            _ => Err(Error::new(
                Status::InvalidArg,
                "set_source_region only supported on PiP surfaces",
            )),
        }
    }

    fn clear_source_region(&self) -> Result<()> {
        match self {
            SurfaceHandle::Pip(p) => {
                p.clear_source_region();
                Ok(())
            }
            _ => Err(Error::new(
                Status::InvalidArg,
                "clear_source_region only supported on PiP surfaces",
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// napi option structs
// ---------------------------------------------------------------------------

#[napi(object)]
pub struct HudOptions {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

#[napi(object)]
pub struct PanelOptions {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub draggable: Option<bool>,
    pub drag_height: Option<u32>,
}

#[napi(object)]
pub struct PipOptions {
    pub source_hwnd: i64,
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

#[napi(object)]
pub struct TrayOptions {
    pub icon_path: Option<String>,
    pub tooltip: Option<String>,
}

#[napi(object)]
pub struct TextOptions {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    pub color: Option<String>,
    pub font_family: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub interactive: Option<bool>,
}

#[napi(object)]
pub struct RectOptions {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fill: Option<String>,
    pub corner_radius: Option<f64>,
    pub border_color: Option<String>,
    pub border_width: Option<f64>,
    pub interactive: Option<bool>,
}

#[napi(object)]
pub struct ImageOptions {
    pub path: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub interactive: Option<bool>,
}

#[napi(object)]
pub struct MenuItemOptions {
    pub id: u32,
    pub label: String,
    pub enabled: Option<bool>,
}

#[napi(object)]
pub struct SourceRegionOptions {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[napi(object)]
pub struct WinPaneEvent {
    pub event_type: String,
    pub surface_id: Option<u32>,
    pub key: Option<String>,
    pub button: Option<String>,
    pub item_id: Option<u32>,
}

// ---------------------------------------------------------------------------
// WinPane class
// ---------------------------------------------------------------------------

#[napi]
pub struct WinPane {
    ctx: Option<winpane::Context>,
    surfaces: HashMap<u32, SurfaceHandle>,
    trays: HashMap<u32, winpane::Tray>,
    surface_id_map: HashMap<u64, u32>,
    next_id: u32,
}

#[napi]
impl WinPane {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        let ctx = winpane::Context::new().map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("failed to create context: {e}"),
            )
        })?;
        Ok(WinPane {
            ctx: Some(ctx),
            surfaces: HashMap::new(),
            trays: HashMap::new(),
            surface_id_map: HashMap::new(),
            next_id: 1,
        })
    }

    fn ctx(&self) -> Result<&winpane::Context> {
        self.ctx
            .as_ref()
            .ok_or_else(|| Error::new(Status::GenericFailure, "context is closed"))
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // -- Surface creation ---------------------------------------------------

    #[napi]
    pub fn create_hud(&mut self, options: HudOptions) -> Result<u32> {
        let config = HudConfig {
            x: options.x.unwrap_or(0),
            y: options.y.unwrap_or(0),
            width: options.width,
            height: options.height,
        };
        let hud = self
            .ctx()?
            .create_hud(config)
            .map_err(|e| Error::new(Status::GenericFailure, format!("create_hud failed: {e}")))?;
        let id = self.next_id();
        self.surface_id_map.insert(hud.id().0, id);
        self.surfaces.insert(id, SurfaceHandle::Hud(hud));
        Ok(id)
    }

    #[napi]
    pub fn create_panel(&mut self, options: PanelOptions) -> Result<u32> {
        let config = PanelConfig {
            x: options.x.unwrap_or(0),
            y: options.y.unwrap_or(0),
            width: options.width,
            height: options.height,
            draggable: options.draggable.unwrap_or(false),
            drag_height: options.drag_height.unwrap_or(0),
        };
        let panel = self
            .ctx()?
            .create_panel(config)
            .map_err(|e| Error::new(Status::GenericFailure, format!("create_panel failed: {e}")))?;
        let id = self.next_id();
        self.surface_id_map.insert(panel.id().0, id);
        self.surfaces.insert(id, SurfaceHandle::Panel(panel));
        Ok(id)
    }

    #[napi]
    pub fn create_pip(&mut self, options: PipOptions) -> Result<u32> {
        let config = PipConfig {
            source_hwnd: options.source_hwnd as isize,
            x: options.x.unwrap_or(0),
            y: options.y.unwrap_or(0),
            width: options.width,
            height: options.height,
        };
        let pip = self
            .ctx()?
            .create_pip(config)
            .map_err(|e| Error::new(Status::GenericFailure, format!("create_pip failed: {e}")))?;
        let id = self.next_id();
        self.surface_id_map.insert(pip.id().0, id);
        self.surfaces.insert(id, SurfaceHandle::Pip(pip));
        Ok(id)
    }

    #[napi]
    pub fn create_tray(&mut self, options: TrayOptions) -> Result<u32> {
        let tooltip = options.tooltip.unwrap_or_default();
        let (icon_rgba, icon_width, icon_height) = match options.icon_path {
            Some(ref path) => load_image_rgba(path, false)?,
            None => {
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
            .ctx()?
            .create_tray(config)
            .map_err(|e| Error::new(Status::GenericFailure, format!("create_tray failed: {e}")))?;
        let id = self.next_id();
        self.trays.insert(id, tray);
        Ok(id)
    }

    // -- Element operations -------------------------------------------------

    #[napi]
    pub fn set_text(&self, surface_id: u32, key: String, options: TextOptions) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        let color = color_or_default(options.color, Color::WHITE)?;
        let element = TextElement {
            text: options.text,
            x: options.x as f32,
            y: options.y as f32,
            font_size: options.font_size as f32,
            color,
            font_family: options.font_family,
            bold: options.bold.unwrap_or(false),
            italic: options.italic.unwrap_or(false),
            interactive: options.interactive.unwrap_or(false),
        };
        surface.set_text(&key, element)
    }

    #[napi]
    pub fn set_rect(&self, surface_id: u32, key: String, options: RectOptions) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        let fill = color_or_default(options.fill, Color::WHITE)?;
        let border_color = match options.border_color {
            Some(ref s) => Some(parse_color(s)?),
            None => None,
        };
        let element = RectElement {
            x: options.x as f32,
            y: options.y as f32,
            width: options.width as f32,
            height: options.height as f32,
            fill,
            corner_radius: options.corner_radius.unwrap_or(0.0) as f32,
            border_color,
            border_width: options.border_width.unwrap_or(0.0) as f32,
            interactive: options.interactive.unwrap_or(false),
        };
        surface.set_rect(&key, element)
    }

    #[napi]
    pub fn set_image(&self, surface_id: u32, key: String, options: ImageOptions) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        let (data, data_width, data_height) = load_image_rgba(&options.path, true)?;
        let element = ImageElement {
            x: options.x as f32,
            y: options.y as f32,
            width: options.width as f32,
            height: options.height as f32,
            data,
            data_width,
            data_height,
            interactive: options.interactive.unwrap_or(false),
        };
        surface.set_image(&key, element)
    }

    #[napi]
    pub fn remove_element(&self, surface_id: u32, key: String) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.remove(&key)
    }

    // -- Surface control ----------------------------------------------------

    #[napi]
    pub fn show(&self, surface_id: u32) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.show();
        Ok(())
    }

    #[napi]
    pub fn hide(&self, surface_id: u32) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.hide();
        Ok(())
    }

    #[napi]
    pub fn set_position(&self, surface_id: u32, x: i32, y: i32) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.set_position(x, y);
        Ok(())
    }

    #[napi]
    pub fn set_size(&self, surface_id: u32, width: u32, height: u32) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.set_size(width, height);
        Ok(())
    }

    #[napi]
    pub fn set_opacity(&self, surface_id: u32, opacity: f64) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.set_opacity(opacity as f32);
        Ok(())
    }

    #[napi]
    pub fn fade_in(&self, surface_id: u32, duration_ms: u32) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.fade_in(duration_ms);
        Ok(())
    }

    #[napi]
    pub fn fade_out(&self, surface_id: u32, duration_ms: u32) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.fade_out(duration_ms);
        Ok(())
    }

    #[napi]
    pub fn set_capture_excluded(&self, surface_id: u32, excluded: bool) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.set_capture_excluded(excluded);
        Ok(())
    }

    // -- Backdrop -----------------------------------------------------------

    #[napi]
    pub fn set_backdrop(&self, surface_id: u32, backdrop: String) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        let backdrop = match backdrop.as_str() {
            "none" => Backdrop::None,
            "mica" => Backdrop::Mica,
            "acrylic" => Backdrop::Acrylic,
            _ => {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("invalid backdrop: {backdrop} (expected none, mica, acrylic)"),
                ))
            }
        };
        surface.set_backdrop(backdrop);
        Ok(())
    }

    #[napi]
    pub fn backdrop_supported(&self) -> bool {
        winpane::backdrop_supported()
    }

    // -- Anchoring ----------------------------------------------------------

    #[napi]
    pub fn anchor_to(
        &self,
        surface_id: u32,
        target_hwnd: i64,
        anchor: String,
        offset_x: i32,
        offset_y: i32,
    ) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        let anchor = parse_anchor(&anchor)?;
        surface.anchor_to(target_hwnd as isize, anchor, (offset_x, offset_y));
        Ok(())
    }

    #[napi]
    pub fn unanchor(&self, surface_id: u32) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.unanchor();
        Ok(())
    }

    // -- PiP-specific -------------------------------------------------------

    #[napi]
    pub fn set_source_region(&self, surface_id: u32, options: SourceRegionOptions) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        let rect = SourceRect {
            x: options.x,
            y: options.y,
            width: options.width,
            height: options.height,
        };
        surface.set_source_region(rect)
    }

    #[napi]
    pub fn clear_source_region(&self, surface_id: u32) -> Result<()> {
        let surface = self.get_surface(surface_id)?;
        surface.clear_source_region()
    }

    // -- Tray methods -------------------------------------------------------

    #[napi]
    pub fn set_tooltip(&self, tray_id: u32, tooltip: String) -> Result<()> {
        let tray = self.get_tray(tray_id)?;
        tray.set_tooltip(&tooltip);
        Ok(())
    }

    #[napi]
    pub fn set_tray_icon(&self, tray_id: u32, icon_path: String) -> Result<()> {
        let tray = self.get_tray(tray_id)?;
        let (rgba, width, height) = load_image_rgba(&icon_path, false)?;
        tray.set_icon(rgba, width, height);
        Ok(())
    }

    #[napi]
    pub fn set_popup(&self, tray_id: u32, panel_surface_id: u32) -> Result<()> {
        let panel = match self.surfaces.get(&panel_surface_id) {
            Some(SurfaceHandle::Panel(p)) => p,
            Some(_) => {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("{panel_surface_id} is not a Panel"),
                ))
            }
            None => {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("unknown surface_id: {panel_surface_id}"),
                ))
            }
        };
        let tray = self.get_tray(tray_id)?;
        tray.set_popup(panel);
        Ok(())
    }

    #[napi]
    pub fn set_menu(&self, tray_id: u32, items: Vec<MenuItemOptions>) -> Result<()> {
        let tray = self.get_tray(tray_id)?;
        let menu_items: Vec<MenuItem> = items
            .into_iter()
            .map(|item| MenuItem {
                id: item.id,
                label: item.label,
                enabled: item.enabled.unwrap_or(true),
            })
            .collect();
        tray.set_menu(menu_items);
        Ok(())
    }

    // -- Events -------------------------------------------------------------

    #[napi]
    pub fn poll_event(&self) -> Result<Option<WinPaneEvent>> {
        let ctx = self.ctx()?;
        match ctx.poll_event() {
            None => Ok(None),
            Some(event) => Ok(Some(self.convert_event(&event))),
        }
    }

    // -- Lifecycle ----------------------------------------------------------

    #[napi]
    pub fn destroy(&mut self, id: u32) -> Result<()> {
        if let Some(handle) = self.surfaces.remove(&id) {
            self.surface_id_map.remove(&handle.id().0);
            drop(handle);
            return Ok(());
        }
        if self.trays.remove(&id).is_some() {
            return Ok(());
        }
        Err(Error::new(Status::InvalidArg, format!("unknown id: {id}")))
    }

    #[napi]
    pub fn close(&mut self) {
        self.surfaces.clear();
        self.trays.clear();
        self.surface_id_map.clear();
        self.ctx.take();
    }

    // -- Private helpers ----------------------------------------------------

    fn get_surface(&self, id: u32) -> Result<&SurfaceHandle> {
        self.surfaces
            .get(&id)
            .ok_or_else(|| Error::new(Status::InvalidArg, format!("unknown surface_id: {id}")))
    }

    fn get_tray(&self, id: u32) -> Result<&winpane::Tray> {
        self.trays
            .get(&id)
            .ok_or_else(|| Error::new(Status::InvalidArg, format!("unknown tray_id: {id}")))
    }

    fn convert_event(&self, event: &winpane::Event) -> WinPaneEvent {
        match event {
            winpane::Event::ElementClicked { surface_id, key } => WinPaneEvent {
                event_type: "element_clicked".to_string(),
                surface_id: self.surface_id_map.get(&surface_id.0).copied(),
                key: Some(key.clone()),
                button: None,
                item_id: None,
            },
            winpane::Event::ElementHovered { surface_id, key } => WinPaneEvent {
                event_type: "element_hovered".to_string(),
                surface_id: self.surface_id_map.get(&surface_id.0).copied(),
                key: Some(key.clone()),
                button: None,
                item_id: None,
            },
            winpane::Event::ElementLeft { surface_id, key } => WinPaneEvent {
                event_type: "element_left".to_string(),
                surface_id: self.surface_id_map.get(&surface_id.0).copied(),
                key: Some(key.clone()),
                button: None,
                item_id: None,
            },
            winpane::Event::TrayClicked { button } => {
                let button_str = match button {
                    MouseButton::Left => "left",
                    MouseButton::Right => "right",
                    MouseButton::Middle => "middle",
                };
                WinPaneEvent {
                    event_type: "tray_clicked".to_string(),
                    surface_id: None,
                    key: None,
                    button: Some(button_str.to_string()),
                    item_id: None,
                }
            }
            winpane::Event::TrayMenuItemClicked { id } => WinPaneEvent {
                event_type: "tray_menu_item_clicked".to_string(),
                surface_id: None,
                key: None,
                button: None,
                item_id: Some(*id),
            },
            winpane::Event::PipSourceClosed { surface_id } => WinPaneEvent {
                event_type: "pip_source_closed".to_string(),
                surface_id: self.surface_id_map.get(&surface_id.0).copied(),
                key: None,
                button: None,
                item_id: None,
            },
            winpane::Event::AnchorTargetClosed { surface_id } => WinPaneEvent {
                event_type: "anchor_target_closed".to_string(),
                surface_id: self.surface_id_map.get(&surface_id.0).copied(),
                key: None,
                button: None,
                item_id: None,
            },
            winpane::Event::DeviceRecovered => WinPaneEvent {
                event_type: "device_recovered".to_string(),
                surface_id: None,
                key: None,
                button: None,
                item_id: None,
            },
        }
    }
}
