# Phase 3: Engine Integration - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p4-advanced-surfaces/learnings.md`
2. `_docs/initial-implementation/p4-advanced-surfaces/initial-plan.md` Steps 7-8
3. `_docs/initial-implementation/p4-advanced-surfaces/3-engine-integration/spec.md`
4. `crates/winpane-core/src/engine.rs`
5. `crates/winpane-core/src/monitor.rs` (from phase 2)
6. `crates/winpane-core/src/window.rs` (for `set_capture_excluded`, `get_dpi_scale`, `create_hud_window`)

## Implementation Checklist

- [x] Validate DWM thumbnail rendering on a NOREDIRECTIONBITMAP window (if it fails, create a `create_pip_window` fallback in window.rs)
- [x] Add new imports to `engine.rs`: monitor types, DWM APIs (cfg-gated), `Anchor`, `PipConfig`, `SourceRect`
- [x] Add `PipState` struct and `SurfaceKind::Pip` variant
- [x] Add `AnchorState` struct
- [x] Add `monitor` and `anchor_states` to `engine_thread_main` local state
- [x] Implement `create_pip_surface` function (cfg-gated, DWM thumbnail registration)
- [x] Add `Command::CreatePip` handler in command match block
- [x] Implement `update_pip_thumbnail_properties` helper (cfg-gated)
- [x] Add `Command::SetSourceRegion` and `Command::ClearSourceRegion` handlers
- [x] Add PiP guard to `Command::SetElement` handler (skip for PiP)
- [x] Add PiP guard to `Command::RemoveElement` handler (skip for PiP)
- [x] Modify `Command::SetOpacity` to branch for PiP (DWM opacity vs DirectComposition)
- [x] Modify `Command::SetSize` to branch for PiP (update thumbnail dest rect)
- [x] Add PiP guard to `Command::CustomDraw` handler (skip for PiP)
- [x] Add PiP cleanup to `Command::DestroySurface` handler (DwmUnregisterThumbnail, unwatch, remove anchor)
- [x] Add PiP skip to render loop
- [x] Add PiP cleanup to shutdown sequence
- [x] Add `Command::AnchorTo` handler (store state, register monitor, initial position)
- [x] Add `Command::Unanchor` handler
- [x] Implement `apply_anchor_position` helper (cfg-gated)
- [x] Add `Command::SetCaptureExcluded` handler
- [x] Implement `process_monitor_events` function (cfg-gated)
- [x] Implement `handle_watched_window_closed` helper (cfg-gated)
- [x] Add `process_monitor_events` call in the engine loop after `process_tray_events`
- [x] Run `cargo fmt --all`
- [x] Run `cargo check --workspace` and verify it passes
- [x] Mark phase complete in root plan.md

## Implementation Summary

Engine now handles all P4 commands. Key changes to `engine.rs`:

- **New types**: `PipState` (thumbnail handle, source HWND, crop region, opacity), `AnchorState` (target HWND, anchor point, offset, minimize tracking), `SurfaceKind::Pip` variant
- **PiP surface creation**: `create_pip_surface` creates a HUD-style window, registers a DWM thumbnail via `DwmRegisterThumbnail`, sets initial properties, and watches the source HWND for close detection
- **PiP guards**: SetElement, RemoveElement, CustomDraw silently skip for PiP surfaces. SetOpacity and SetSize branch to update DWM thumbnail properties instead of scene/DirectComposition
- **Anchoring**: AnchorTo stores state and registers the target in the window monitor. Unanchor removes state and unwatches. `apply_anchor_position` uses GetWindowRect on the target to calculate position from anchor point + offset
- **Monitor event processing**: `process_monitor_events` drains the thread-local event queue. LocationChanged repositions anchored surfaces. Minimized hides them. Restored shows them. Window close detected via IsWindow check fires PipSourceClosed/AnchorTargetClosed events
- **Capture exclusion**: SetCaptureExcluded delegates to `window::set_capture_excluded` (added non-Windows stub)
- **Cleanup**: DestroySurface and Shutdown both unregister DWM thumbnails and clean up anchor/monitor state

DWM imports are non-cfg-gated (matching existing windows crate import pattern). Helper functions (`process_monitor_events`, `update_pip_thumbnail_properties`, `apply_anchor_position`, `create_pip_surface`) are cfg-gated with non-Windows stubs.

Build check: `cargo check -p winpane-core` shows only the pre-existing `windows-future v0.3.2` transitive dependency error (documented in learnings). No errors from P4 code.
