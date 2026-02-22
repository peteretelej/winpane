// winpane: Public API for Windows companion surfaces (overlays, HUDs, panels).
//
// All Win32 calls stay in winpane-core. This crate is pure Rust wrapping EngineHandle.

pub use winpane_core::{Color, Error, HudConfig, ImageElement, RectElement, TextElement};

use std::sync::mpsc;
use winpane_core::{Command, CommandSender, Element, EngineHandle, SendHwnd, SurfaceId};

// --- Context ---

/// Top-level entry point. Spawns the engine thread and manages its lifetime.
pub struct Context {
    engine: EngineHandle,
}

impl Context {
    /// Create a new winpane context. Spawns the background engine thread
    /// that owns the Win32 message loop and GPU resources.
    pub fn new() -> Result<Self, Error> {
        let engine = EngineHandle::spawn()?;
        Ok(Context { engine })
    }

    /// Create a HUD overlay surface at the given position and size.
    /// Blocks until the engine thread creates the window and GPU resources.
    pub fn create_hud(&self, config: HudConfig) -> Result<Hud, Error> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.engine
            .sender
            .send(Command::CreateHud {
                config,
                reply: reply_tx,
            })
            .map_err(|_| Error::Shutdown)?;

        self.engine.wake();

        reply_rx.recv().map_err(|_| Error::Shutdown)?.map(|id| Hud {
            id,
            sender: self.engine.sender.clone(),
            control_hwnd: self.engine.control_hwnd,
        })
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

    /// Send a command and wake the engine. Silently ignores errors (channel closed = shutdown).
    fn send(&self, cmd: Command) {
        let _ = self.sender.send(cmd);
        // Safety: wake_engine uses PostMessageW which is thread-safe by Win32 spec.
        // SendHwnd wraps HWND to allow cross-thread use.
        wake_engine(self.control_hwnd);
    }
}

impl Drop for Hud {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::DestroySurface(self.id));
        wake_engine(self.control_hwnd);
    }
}

// --- wake helper ---

fn wake_engine(control_hwnd: SendHwnd) {
    winpane_core::wake_engine(control_hwnd);
}
