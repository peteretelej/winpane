use std::sync::mpsc;

use crate::scene::Element;
use crate::types::{
    Anchor, Backdrop, DrawOp, Error, HudConfig, MenuItem, PanelConfig, PipConfig, SourceRect,
    SurfaceId, TrayConfig, TrayId,
};

pub enum Command {
    // --- Surface lifecycle ---
    CreateHud {
        config: HudConfig,
        reply: mpsc::Sender<Result<SurfaceId, Error>>,
    },
    CreatePanel {
        config: PanelConfig,
        reply: mpsc::Sender<Result<SurfaceId, Error>>,
    },
    CreatePip {
        config: PipConfig,
        reply: mpsc::Sender<Result<SurfaceId, Error>>,
    },
    DestroySurface(SurfaceId),
    Shutdown,

    // --- Scene graph ---
    SetElement {
        surface: SurfaceId,
        key: String,
        element: Element,
    },
    RemoveElement {
        surface: SurfaceId,
        key: String,
    },

    // --- Surface properties ---
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
    SetBackdrop {
        surface: SurfaceId,
        backdrop: Backdrop,
    },
    FadeIn {
        surface: SurfaceId,
        duration_ms: u32,
    },
    FadeOut {
        surface: SurfaceId,
        duration_ms: u32,
    },

    // --- Tray ---
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

    // --- Custom draw ---
    CustomDraw {
        surface: SurfaceId,
        ops: Vec<DrawOp>,
    },

    // --- Window tracking ---
    SetSourceRegion {
        surface: SurfaceId,
        rect: SourceRect,
    },
    ClearSourceRegion {
        surface: SurfaceId,
    },
    AnchorTo {
        surface: SurfaceId,
        target_hwnd: isize,
        anchor: Anchor,
        offset: (i32, i32),
    },
    Unanchor {
        surface: SurfaceId,
    },
    SetCaptureExcluded {
        surface: SurfaceId,
        excluded: bool,
    },
    GetPosition {
        surface: SurfaceId,
        reply: mpsc::Sender<Result<(i32, i32), Error>>,
    },
}

pub type CommandSender = mpsc::Sender<Command>;
pub type CommandReceiver = mpsc::Receiver<Command>;
