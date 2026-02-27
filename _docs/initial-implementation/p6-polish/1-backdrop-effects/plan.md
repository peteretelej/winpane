# Phase 1: Backdrop Effects - Plan

## Required Reading

- `_docs/initial-implementation/p6-polish/learnings.md`
- `_docs/initial-implementation/p6-polish/1-backdrop-effects/spec.md`
- `_docs/initial-implementation/p6-polish/initial-plan.md` (Steps 1-3)

## Implementation Checklist

- [x] Add `Backdrop` enum to `crates/winpane-core/src/types.rs`
- [x] Add `SetBackdrop` command variant to `crates/winpane-core/src/command.rs`
- [x] Add `Backdrop` to re-exports in `crates/winpane-core/src/lib.rs`
- [x] Add `"Win32_UI_Controls"` to windows features in `crates/winpane-core/Cargo.toml` (provides `MARGINS` struct)
- [x] Add `supports_backdrop()` and `set_window_backdrop()` to `crates/winpane-core/src/window.rs`
- [x] Add `SetBackdrop` handler in engine command dispatch in `crates/winpane-core/src/engine.rs`
- [x] Add `set_backdrop()` to `Hud`, `Panel`, `Pip` in `crates/winpane/src/lib.rs`; add `Backdrop` to re-exports
- [x] Add `pub fn backdrop_supported() -> bool` to `crates/winpane/src/lib.rs` (delegates to winpane-core)
- [x] Add `pub fn backdrop_supported() -> bool` to `crates/winpane-core/src/lib.rs` (delegates to window::supports_backdrop)
- [x] Add `WinpaneBackdrop` enum, `FfiSurface::set_backdrop()`, `winpane_surface_set_backdrop` and `winpane_backdrop_supported` extern fns in `crates/winpane-ffi/src/lib.rs`
- [x] Add `set_backdrop` and `backdrop_supported` to dispatch in `crates/winpane-host/src/dispatch.rs`
- [x] Add `set_backdrop` and `backdrop_supported` to `WinPane` napi class in `bindings/node/src/lib.rs`
- [x] Run `cargo fmt --all` and verify `cargo fmt --all -- --check` passes
- [x] Mark phase complete

## Implementation Summary

Added DWM backdrop effects (Mica, Acrylic) across all API layers. The implementation uses `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)` with `DwmExtendFrameIntoClientArea` frame extension, gated by a build number check (Win11 22H2+ / build 22621). Silent no-op on older Windows versions. Exposed `Backdrop` enum, `set_backdrop()` surface method, and `backdrop_supported()` query through Rust API, C ABI, JSON-RPC host, and Node.js addon.
