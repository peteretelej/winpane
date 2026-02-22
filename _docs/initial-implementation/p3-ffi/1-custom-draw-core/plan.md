# Phase 1: Custom Draw Core - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p3-ffi/learnings.md`
2. `_docs/initial-implementation/p3-ffi/initial-plan.md` Phases 1-2
3. `_docs/initial-implementation/p3-ffi/1-custom-draw-core/spec.md`
4. `crates/winpane-core/src/types.rs`
5. `crates/winpane-core/src/command.rs`
6. `crates/winpane-core/src/renderer.rs`
7. `crates/winpane-core/src/engine.rs`
8. `crates/winpane/src/lib.rs`

## Implementation Checklist

- [x] Add `DrawOp` enum to `crates/winpane-core/src/types.rs` after `ImageElement`
- [x] Add `DrawOp` import and `Command::CustomDraw` variant to `crates/winpane-core/src/command.rs`
- [x] Add `execute_draw_ops` and `execute_single_draw_op` methods to `SurfaceRenderer` in `crates/winpane-core/src/renderer.rs`
- [x] Add `Command::CustomDraw` handler in `crates/winpane-core/src/engine.rs`
- [x] Verify `DrawOp` is re-exported from `crates/winpane-core/src/lib.rs` (automatic via `pub use types::*`)
- [x] Add `DrawOp` to re-export list and `custom_draw()` to Hud and Panel in `crates/winpane/src/lib.rs`
- [x] Create `examples/rust/custom_draw.rs` with bar chart demo
- [x] Register `custom_draw` example in `crates/winpane/Cargo.toml`
- [x] Run `cargo fmt --all`
- [x] Mark phase complete in root plan.md

## Implementation Summary

Added custom draw escape hatch to winpane. The `DrawOp` enum (10 variants: clear, fill/stroke rect, text, line, fill/stroke ellipse, image, fill/stroke rounded rect) allows rendering beyond the declarative scene graph. `execute_draw_ops` performs a full BeginDraw/EndDraw/Present cycle, rendering the retained-mode scene first then custom ops on top. Used `windows_numerics::Vector2` for D2D point types per the windows-rs 0.62 migration (D2D_POINT_2F replaced). Custom draw is one-shot by design: the next scene graph change overwrites custom draw content.
