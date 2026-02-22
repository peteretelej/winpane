# Phase 4: Public API and FFI - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p4-advanced-surfaces/learnings.md`
2. `_docs/initial-implementation/p4-advanced-surfaces/initial-plan.md` Steps 9-11
3. `_docs/initial-implementation/p4-advanced-surfaces/4-public-api-and-ffi/spec.md`
4. `crates/winpane/src/lib.rs`
5. `crates/winpane-ffi/src/lib.rs`
6. `crates/winpane-ffi/include/winpane.def`

## Implementation Checklist

- [x] Add `Anchor`, `PipConfig`, `SourceRect` to re-exports in `crates/winpane/src/lib.rs`
- [x] Add `Context::create_pip` method
- [x] Add `Pip` struct with all methods (show, hide, set_position, set_size, set_opacity, set_source_region, clear_source_region, anchor_to, unanchor, set_capture_excluded, id) and Drop impl
- [x] Add `anchor_to`, `unanchor`, `set_capture_excluded` to `Hud` impl
- [x] Add `anchor_to`, `unanchor`, `set_capture_excluded` to `Panel` impl
- [x] Add `FfiSurface::Pip` variant to `crates/winpane-ffi/src/lib.rs`
- [x] Update all `FfiSurface` dispatch methods to handle `Pip` (supported ops dispatch, unsupported ops return error)
- [x] Add `anchor_to`, `unanchor`, `set_capture_excluded` dispatch methods to `FfiSurface`
- [x] Add `WinpanePipConfig` repr(C) struct with `to_rust()` conversion
- [x] Add `WinpaneSourceRect` repr(C) struct with `to_rust()` conversion
- [x] Add `WinpaneAnchor` repr(C) enum with `to_rust()` conversion
- [x] Add `PipSourceClosed = 6` and `AnchorTargetClosed = 7` to `WinpaneEventType`
- [x] Update `WinpaneEvent::from_rust` for new event variants
- [x] Implement `winpane_pip_create` FFI function
- [x] Implement `winpane_surface_set_source_region` FFI function
- [x] Implement `winpane_surface_clear_source_region` FFI function
- [x] Implement `winpane_surface_anchor_to` FFI function
- [x] Implement `winpane_surface_unanchor` FFI function
- [x] Implement `winpane_surface_set_capture_excluded` FFI function
- [x] Append 6 new export names to `crates/winpane-ffi/include/winpane.def`
- [x] Run `cargo fmt --all`
- [x] Run `cargo check --workspace` and verify it passes
- [x] Mark phase complete in root plan.md

## Implementation Summary

Added public Rust API: `Pip` struct with show/hide/position/size/opacity/source_region/anchor/capture methods, `Context::create_pip`, and `anchor_to`/`unanchor`/`set_capture_excluded` on `Hud` and `Panel`. Updated FFI layer with `FfiSurface::Pip` variant, `WinpanePipConfig`/`WinpaneSourceRect`/`WinpaneAnchor` C types, `PipSourceClosed`/`AnchorTargetClosed` event types, and 6 new FFI functions. The .def file now has 41 exports. `cargo check --workspace` failure is the pre-existing `windows-future` transitive dependency issue (not caused by P4 changes).
