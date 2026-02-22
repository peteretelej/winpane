// winpane-core: Internal Win32/DirectComposition logic.
// Public API is exposed through the `winpane` crate.

pub(crate) mod command;
pub(crate) mod engine;
pub(crate) mod input;
pub(crate) mod monitor;
pub(crate) mod renderer;
pub(crate) mod scene;
pub(crate) mod tray;
pub mod types;
pub(crate) mod window;

// Re-export public types for end users (via winpane crate)
pub use types::*;

// Bridge types for the winpane crate (not exposed to end users)
pub use command::{Command, CommandSender};
pub use engine::{wake_engine, EngineHandle};
pub use scene::Element;
pub use types::SurfaceId;
pub use window::SendHwnd;
