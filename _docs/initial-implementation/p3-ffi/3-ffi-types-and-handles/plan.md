# Phase 3: FFI Types and Handles - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p3-ffi/learnings.md`
2. `_docs/initial-implementation/p3-ffi/initial-plan.md` Phases 5-6
3. `_docs/initial-implementation/p3-ffi/3-ffi-types-and-handles/spec.md`
4. `crates/winpane-ffi/src/lib.rs` (current state after Phase 2)
5. `crates/winpane/src/lib.rs` (for Rust type signatures to match)

## Implementation Checklist

- [ ] Add `WINPANE_CONFIG_VERSION` constant and `WinpaneColor` struct with `to_rust()` conversion
- [ ] Add versioned config structs: `WinpaneHudConfig`, `WinpanePanelConfig`, `WinpaneTrayConfig` with `to_rust()` methods
- [ ] Add element structs: `WinpaneTextElement`, `WinpaneRectElement`, `WinpaneImageElement` with `to_rust()` methods
- [ ] Add `WinpaneMenuItem` struct
- [ ] Add event types: `WinpaneEventType` enum, `WinpaneMouseButton` enum, `WinpaneEvent` struct with `from_rust()` and `copy_key_to_buffer()`
- [ ] Add opaque handle types: `FfiSurface` enum with dispatch methods, `WinpaneContext`, `WinpaneSurface`, `WinpaneTray`, `CanvasAccumulator`, `WinpaneCanvas`
- [ ] Implement `winpane_create()` and `winpane_destroy()`
- [ ] Implement `winpane_poll_event()`
- [ ] Verify `cargo build --workspace` succeeds and `winpane.h` contains type definitions
- [ ] Run `cargo fmt --all`
- [ ] Mark phase complete in root plan.md

## Implementation Summary

*(To be filled after implementation)*
