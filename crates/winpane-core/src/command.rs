use std::sync::mpsc;

use crate::scene::Element;
use crate::types::{Error, HudConfig, MenuItem, PanelConfig, SurfaceId, TrayConfig, TrayId};

pub enum Command {
    // --- Existing P1 commands ---
    CreateHud {
        config: HudConfig,
        reply: mpsc::Sender<Result<SurfaceId, Error>>,
    },
    SetElement {
        surface: SurfaceId,
        key: String,
        element: Element,
    },
    RemoveElement {
        surface: SurfaceId,
        key: String,
    },
    Show(SurfaceId),
    Hide(SurfaceId),
    SetPosition {
        surface: SurfaceId,
        x: i32,
        y: i32,
    },
    SetSize {
        surface: SurfaceId,
        width: u32,
        height: u32,
    },
    SetOpacity {
        surface: SurfaceId,
        opacity: f32,
    },
    DestroySurface(SurfaceId),
    Shutdown,

    // --- New P2 commands ---
    CreatePanel {
        config: PanelConfig,
        reply: mpsc::Sender<Result<SurfaceId, Error>>,
    },
    CreateTray {
        config: TrayConfig,
        reply: mpsc::Sender<Result<TrayId, Error>>,
    },
    SetTrayTooltip {
        tray: TrayId,
        tooltip: String,
    },
    SetTrayIcon {
        tray: TrayId,
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    },
    SetTrayPopup {
        tray: TrayId,
        surface: SurfaceId,
    },
    SetTrayMenu {
        tray: TrayId,
        items: Vec<MenuItem>,
    },
    DestroyTray(TrayId),
}

pub type CommandSender = mpsc::Sender<Command>;
pub type CommandReceiver = mpsc::Receiver<Command>;
