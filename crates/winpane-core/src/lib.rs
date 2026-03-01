// winpane-core: Internal Win32/DirectComposition logic.
// Public API is exposed through the `winpane` crate.

pub(crate) mod command;
pub(crate) mod display;
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

// Re-export display enumeration for public API
pub use display::enumerate_monitors;

/// Returns true if the current Windows build supports DWM backdrop effects (Win11 22H2+).
#[cfg(target_os = "windows")]
pub fn backdrop_supported() -> bool {
    window::supports_backdrop()
}

#[cfg(not(target_os = "windows"))]
pub fn backdrop_supported() -> bool {
    false
}

// Bridge types for the winpane crate (not exposed to end users)
pub use command::{Command, CommandSender};
pub use engine::{EngineHandle, wake_engine};
pub use scene::Element;
pub use types::SurfaceId;
pub use window::SendHwnd;
