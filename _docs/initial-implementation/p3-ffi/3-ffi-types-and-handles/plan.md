# Phase 3: FFI Types and Handles - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p3-ffi/learnings.md`
2. `_docs/initial-implementation/p3-ffi/initial-plan.md` Phases 5-6
3. `_docs/initial-implementation/p3-ffi/3-ffi-types-and-handles/spec.md`
4. `crates/winpane-ffi/src/lib.rs` (current state after Phase 2)
5. `crates/winpane/src/lib.rs` (for Rust type signatures to match)

## Implementation Checklist

- [x] Add `WINPANE_CONFIG_VERSION` constant and `WinpaneColor` struct with `to_rust()` conversion
- [x] Add versioned config structs: `WinpaneHudConfig`, `WinpanePanelConfig`, `WinpaneTrayConfig` with `to_rust()` methods
- [x] Add element structs: `WinpaneTextElement`, `WinpaneRectElement`, `WinpaneImageElement` with `to_rust()` methods
- [x] Add `WinpaneMenuItem` struct
- [x] Add event types: `WinpaneEventType` enum, `WinpaneMouseButton` enum, `WinpaneEvent` struct with `from_rust()` and `copy_key_to_buffer()`
- [x] Add opaque handle types: `FfiSurface` enum with dispatch methods, `WinpaneContext`, `WinpaneSurface`, `WinpaneTray`, `CanvasAccumulator`, `WinpaneCanvas`
- [x] Implement `winpane_create()` and `winpane_destroy()`
- [x] Implement `winpane_poll_event()`
- [x] Verify `cargo build --workspace` succeeds and `winpane.h` contains type definitions
- [x] Run `cargo fmt --all`
- [x] Mark phase complete in root plan.md

## Implementation Summary

All repr(C) types, opaque handle types, and context/event functions added to `crates/winpane-ffi/src/lib.rs`:

- **Config version + color**: `WINPANE_CONFIG_VERSION` constant, `WinpaneColor` with `to_rust()` conversion
- **Versioned configs**: `WinpaneHudConfig`, `WinpanePanelConfig`, `WinpaneTrayConfig` with version and size validation in `to_rust()`
- **Element structs**: `WinpaneTextElement`, `WinpaneRectElement`, `WinpaneImageElement` with conversions handling C strings, nullable pointers, and i32-to-bool mapping
- **Menu item**: `WinpaneMenuItem` (id, label, enabled)
- **Events**: `WinpaneEventType` and `WinpaneMouseButton` enums, `WinpaneEvent` struct with `from_rust()` conversion and `copy_key_to_buffer()` helper (255-byte key limit)
- **Opaque handles**: `FfiSurface` enum (Hud/Panel) with 11 dispatch methods, `WinpaneContext`, `WinpaneSurface` (with canvas slot), `WinpaneTray`, `CanvasAccumulator`, `WinpaneCanvas`
- **Context lifecycle**: `winpane_create()` (boxes context, writes to out-pointer via `ffi_try!`), `winpane_destroy()` (reclaims via `Box::from_raw`)
- **Event polling**: `winpane_poll_event()` with manual `catch_unwind` (returns 0=event, 1=none, -1=error, -2=panic)

Build verified: no errors from winpane-ffi (only expected Windows-only dependency errors on macOS). Code passes `rustfmt --check`.
