// winpane: Public API for Windows companion surfaces (overlays, HUDs, panels).
//
// All Win32 calls stay in winpane-core. This crate is pure Rust wrapping EngineHandle.

pub use winpane_core::{
    Color, Error, Event, HudConfig, ImageElement, MenuItem, MouseButton, PanelConfig, RectElement,
    SurfaceId, TextElement, TrayConfig, TrayId,
};

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

    /// Polls for the next event. Returns None if no events are pending.
    pub fn poll_event(&self) -> Option<Event> {
        self.event_rx.try_recv().ok()
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
