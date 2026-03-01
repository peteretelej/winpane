// winpane: Public API for Windows companion surfaces (overlays, HUDs, panels).
//
// All Win32 calls stay in winpane-core. This crate is pure Rust wrapping EngineHandle.

pub use winpane_core::{
    Anchor, Backdrop, Color, DrawOp, Error, Event, HudConfig, ImageElement, MenuItem, MonitorInfo,
    MouseButton, PanelConfig, PipConfig, Placement, RectElement, SourceRect, SurfaceId,
    TextElement, TrayConfig, TrayId,
};

/// Returns true if the current Windows build supports DWM backdrop effects (Win11 22H2+).
pub fn backdrop_supported() -> bool {
    winpane_core::backdrop_supported()
}

use std::sync::mpsc;
use winpane_core::{Command, CommandSender, Element, EngineHandle, SendHwnd};

// --- Context ---

/// Top-level entry point. Spawns the engine thread and manages its lifetime.
pub struct Context {
    engine: EngineHandle,
    event_rx: mpsc::Receiver<Event>,
}

impl Context {
    /// Create a new winpane context. Spawns the background engine thread
    /// that owns the Win32 message loop and GPU resources.
    pub fn new() -> Result<Self, Error> {
        let (engine, event_rx) = EngineHandle::spawn()?;
        Ok(Context { engine, event_rx })
    }

    /// Create a HUD overlay surface at the given position and size.
    /// Blocks until the engine thread creates the window and GPU resources.
    pub fn create_hud(&self, config: HudConfig) -> Result<Hud, Error> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.engine.send_and_wake(Command::CreateHud {
            config,
            reply: reply_tx,
        });
        reply_rx.recv().map_err(|_| Error::Shutdown)?.map(|id| Hud {
            id,
            sender: self.engine.sender.clone(),
            control_hwnd: self.engine.control_hwnd,
        })
    }

    /// Creates an interactive panel surface.
    pub fn create_panel(&self, config: PanelConfig) -> Result<Panel, Error> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.engine.send_and_wake(Command::CreatePanel {
            config,
            reply: reply_tx,
        });
        reply_rx
            .recv()
            .map_err(|_| Error::Shutdown)?
            .map(|id| Panel {
                id,
                sender: self.engine.sender.clone(),
                control_hwnd: self.engine.control_hwnd,
            })
    }

    /// Creates a system tray icon.
    pub fn create_tray(&self, config: TrayConfig) -> Result<Tray, Error> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.engine.send_and_wake(Command::CreateTray {
            config,
            reply: reply_tx,
        });
        reply_rx
            .recv()
            .map_err(|_| Error::Shutdown)?
            .map(|id| Tray {
                id,
                sender: self.engine.sender.clone(),
                control_hwnd: self.engine.control_hwnd,
            })
    }

    /// Creates a PiP (Picture-in-Picture) surface showing a live DWM thumbnail
    /// of the specified source window.
    pub fn create_pip(&self, config: PipConfig) -> Result<Pip, Error> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.engine.send_and_wake(Command::CreatePip {
            config,
            reply: reply_tx,
        });
        reply_rx.recv().map_err(|_| Error::Shutdown)?.map(|id| Pip {
            id,
            sender: self.engine.sender.clone(),
            control_hwnd: self.engine.control_hwnd,
        })
    }

    /// Polls for the next event. Returns None if no events are pending.
    pub fn poll_event(&self) -> Option<Event> {
        self.event_rx.try_recv().ok()
    }

    /// Returns information about all connected display monitors.
    /// The primary monitor is listed first, followed by others sorted left-to-right.
    pub fn monitors(&self) -> Vec<MonitorInfo> {
        winpane_core::enumerate_monitors()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        let _ = self.engine.sender.send(Command::Shutdown);
        self.engine.wake();
        if let Some(handle) = self.engine.join_handle.take() {
            let _ = handle.join();
        }
    }
}

// --- Hud ---

/// A HUD overlay surface. Provides methods to add/remove elements,
/// control visibility, and adjust position/size/opacity.
pub struct Hud {
    id: SurfaceId,
    sender: CommandSender,
    control_hwnd: SendHwnd,
}

impl Hud {
    pub fn set_text(&self, key: &str, element: TextElement) {
        self.send(Command::SetElement {
            surface: self.id,
            key: key.to_string(),
            element: Element::Text(element),
        });
    }

    pub fn set_rect(&self, key: &str, element: RectElement) {
        self.send(Command::SetElement {
            surface: self.id,
            key: key.to_string(),
            element: Element::Rect(element),
        });
    }

    pub fn set_image(&self, key: &str, element: ImageElement) {
        self.send(Command::SetElement {
            surface: self.id,
            key: key.to_string(),
            element: Element::Image(element),
        });
    }

    pub fn remove(&self, key: &str) {
        self.send(Command::RemoveElement {
            surface: self.id,
            key: key.to_string(),
        });
    }

    pub fn show(&self) {
        self.send(Command::Show(self.id));
    }

    pub fn hide(&self) {
        self.send(Command::Hide(self.id));
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.send(Command::SetPosition {
            surface: self.id,
            x,
            y,
        });
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.send(Command::SetSize {
            surface: self.id,
            width,
            height,
        });
    }

    pub fn set_opacity(&self, opacity: f32) {
        self.send(Command::SetOpacity {
            surface: self.id,
            opacity: opacity.clamp(0.0, 1.0),
        });
    }

    pub fn id(&self) -> SurfaceId {
        self.id
    }

    /// Execute custom draw operations on this surface.
    /// Renders the scene graph first, then the provided ops on top.
    /// One-shot: the next scene graph change overwrites custom draw content.
    pub fn custom_draw(&self, ops: Vec<DrawOp>) {
        self.send(Command::CustomDraw {
            surface: self.id,
            ops,
        });
    }

    pub fn anchor_to(&self, target_hwnd: isize, anchor: Anchor, offset: (i32, i32)) {
        self.send(Command::AnchorTo {
            surface: self.id,
            target_hwnd,
            anchor,
            offset,
        });
    }

    pub fn unanchor(&self) {
        self.send(Command::Unanchor { surface: self.id });
    }

    pub fn set_capture_excluded(&self, excluded: bool) {
        self.send(Command::SetCaptureExcluded {
            surface: self.id,
            excluded,
        });
    }

    pub fn set_backdrop(&self, backdrop: Backdrop) {
        self.send(Command::SetBackdrop {
            surface: self.id,
            backdrop,
        });
    }

    pub fn fade_in(&self, duration_ms: u32) {
        self.send(Command::FadeIn {
            surface: self.id,
            duration_ms,
        });
    }

    pub fn fade_out(&self, duration_ms: u32) {
        self.send(Command::FadeOut {
            surface: self.id,
            duration_ms,
        });
    }

    fn send(&self, cmd: Command) {
        let _ = self.sender.send(cmd);
        wake_engine(self.control_hwnd);
    }
}

impl Drop for Hud {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::DestroySurface(self.id));
        wake_engine(self.control_hwnd);
    }
}

// --- Panel ---

/// An interactive panel surface. Same element API as Hud, plus click/hover events.
pub struct Panel {
    id: SurfaceId,
    sender: CommandSender,
    control_hwnd: SendHwnd,
}

impl Panel {
    pub fn set_text(&self, key: &str, element: TextElement) {
        self.send(Command::SetElement {
            surface: self.id,
            key: key.to_string(),
            element: Element::Text(element),
        });
    }

    pub fn set_rect(&self, key: &str, element: RectElement) {
        self.send(Command::SetElement {
            surface: self.id,
            key: key.to_string(),
            element: Element::Rect(element),
        });
    }

    pub fn set_image(&self, key: &str, element: ImageElement) {
        self.send(Command::SetElement {
            surface: self.id,
            key: key.to_string(),
            element: Element::Image(element),
        });
    }

    pub fn remove(&self, key: &str) {
        self.send(Command::RemoveElement {
            surface: self.id,
            key: key.to_string(),
        });
    }

    pub fn show(&self) {
        self.send(Command::Show(self.id));
    }

    pub fn hide(&self) {
        self.send(Command::Hide(self.id));
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.send(Command::SetPosition {
            surface: self.id,
            x,
            y,
        });
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.send(Command::SetSize {
            surface: self.id,
            width,
            height,
        });
    }

    pub fn set_opacity(&self, opacity: f32) {
        self.send(Command::SetOpacity {
            surface: self.id,
            opacity: opacity.clamp(0.0, 1.0),
        });
    }

    pub fn id(&self) -> SurfaceId {
        self.id
    }

    /// Execute custom draw operations on this surface.
    /// Renders the scene graph first, then the provided ops on top.
    /// One-shot: the next scene graph change overwrites custom draw content.
    pub fn custom_draw(&self, ops: Vec<DrawOp>) {
        self.send(Command::CustomDraw {
            surface: self.id,
            ops,
        });
    }

    pub fn anchor_to(&self, target_hwnd: isize, anchor: Anchor, offset: (i32, i32)) {
        self.send(Command::AnchorTo {
            surface: self.id,
            target_hwnd,
            anchor,
            offset,
        });
    }

    pub fn unanchor(&self) {
        self.send(Command::Unanchor { surface: self.id });
    }

    pub fn set_capture_excluded(&self, excluded: bool) {
        self.send(Command::SetCaptureExcluded {
            surface: self.id,
            excluded,
        });
    }

    pub fn set_backdrop(&self, backdrop: Backdrop) {
        self.send(Command::SetBackdrop {
            surface: self.id,
            backdrop,
        });
    }

    pub fn fade_in(&self, duration_ms: u32) {
        self.send(Command::FadeIn {
            surface: self.id,
            duration_ms,
        });
    }

    pub fn fade_out(&self, duration_ms: u32) {
        self.send(Command::FadeOut {
            surface: self.id,
            duration_ms,
        });
    }

    fn send(&self, cmd: Command) {
        let _ = self.sender.send(cmd);
        wake_engine(self.control_hwnd);
    }
}

impl Drop for Panel {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::DestroySurface(self.id));
        wake_engine(self.control_hwnd);
    }
}

// --- Pip ---

/// A Picture-in-Picture surface showing a live DWM thumbnail of another window.
/// Does not support scene graph operations (set_text, set_rect, etc.).
pub struct Pip {
    id: SurfaceId,
    sender: CommandSender,
    control_hwnd: SendHwnd,
}

impl Pip {
    pub fn show(&self) {
        self.send(Command::Show(self.id));
    }

    pub fn hide(&self) {
        self.send(Command::Hide(self.id));
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.send(Command::SetPosition {
            surface: self.id,
            x,
            y,
        });
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.send(Command::SetSize {
            surface: self.id,
            width,
            height,
        });
    }

    pub fn set_opacity(&self, opacity: f32) {
        self.send(Command::SetOpacity {
            surface: self.id,
            opacity: opacity.clamp(0.0, 1.0),
        });
    }

    /// Sets the source window crop region. Only the specified rectangle
    /// of the source window is shown in the thumbnail.
    pub fn set_source_region(&self, rect: SourceRect) {
        self.send(Command::SetSourceRegion {
            surface: self.id,
            rect,
        });
    }

    /// Clears the source crop, showing the full source window.
    pub fn clear_source_region(&self) {
        self.send(Command::ClearSourceRegion { surface: self.id });
    }

    pub fn anchor_to(&self, target_hwnd: isize, anchor: Anchor, offset: (i32, i32)) {
        self.send(Command::AnchorTo {
            surface: self.id,
            target_hwnd,
            anchor,
            offset,
        });
    }

    pub fn unanchor(&self) {
        self.send(Command::Unanchor { surface: self.id });
    }

    pub fn set_capture_excluded(&self, excluded: bool) {
        self.send(Command::SetCaptureExcluded {
            surface: self.id,
            excluded,
        });
    }

    pub fn set_backdrop(&self, backdrop: Backdrop) {
        self.send(Command::SetBackdrop {
            surface: self.id,
            backdrop,
        });
    }

    pub fn fade_in(&self, duration_ms: u32) {
        self.send(Command::FadeIn {
            surface: self.id,
            duration_ms,
        });
    }

    pub fn fade_out(&self, duration_ms: u32) {
        self.send(Command::FadeOut {
            surface: self.id,
            duration_ms,
        });
    }

    pub fn id(&self) -> SurfaceId {
        self.id
    }

    fn send(&self, cmd: Command) {
        let _ = self.sender.send(cmd);
        wake_engine(self.control_hwnd);
    }
}

impl Drop for Pip {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::DestroySurface(self.id));
        wake_engine(self.control_hwnd);
    }
}

// --- Tray ---

/// A system tray icon. Supports tooltip, icon updates, popup panel, and context menu.
pub struct Tray {
    id: TrayId,
    sender: CommandSender,
    control_hwnd: SendHwnd,
}

impl Tray {
    pub fn set_tooltip(&self, tooltip: &str) {
        self.send(Command::SetTrayTooltip {
            tray: self.id,
            tooltip: tooltip.to_string(),
        });
    }

    pub fn set_icon(&self, rgba: Vec<u8>, width: u32, height: u32) {
        self.send(Command::SetTrayIcon {
            tray: self.id,
            rgba,
            width,
            height,
        });
    }

    /// Associates a panel as the tray popup. Left-clicking the tray icon
    /// toggles the panel's visibility.
    pub fn set_popup(&self, panel: &Panel) {
        self.send(Command::SetTrayPopup {
            tray: self.id,
            surface: panel.id(),
        });
    }

    /// Sets the right-click context menu items.
    pub fn set_menu(&self, items: Vec<MenuItem>) {
        self.send(Command::SetTrayMenu {
            tray: self.id,
            items,
        });
    }

    fn send(&self, cmd: Command) {
        let _ = self.sender.send(cmd);
        wake_engine(self.control_hwnd);
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::DestroyTray(self.id));
        wake_engine(self.control_hwnd);
    }
}

// --- wake helper ---

fn wake_engine(control_hwnd: SendHwnd) {
    winpane_core::wake_engine(control_hwnd);
}
