use std::sync::mpsc;

use crate::scene::Element;
use crate::types::{Error, HudConfig, SurfaceId};

pub(crate) enum Command {
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
}

pub(crate) type CommandSender = mpsc::Sender<Command>;
pub(crate) type CommandReceiver = mpsc::Receiver<Command>;
