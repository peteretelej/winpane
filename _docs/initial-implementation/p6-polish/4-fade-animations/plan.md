# Phase 4: Fade Animations - Plan

## Required Reading

- `_docs/initial-implementation/p6-polish/learnings.md`
- `_docs/initial-implementation/p6-polish/4-fade-animations/spec.md`
- `_docs/initial-implementation/p6-polish/initial-plan.md` (Steps 7-9)

## Implementation Checklist

- [x] Add `FadeIn` and `FadeOut` command variants to `crates/winpane-core/src/command.rs`
- [x] Add `animate_opacity()` method to `SurfaceRenderer` in `crates/winpane-core/src/renderer.rs` (uses `IDCompositionEffectGroup::SetOpacity` with animation, NOT `IDCompositionVisual::SetOpacity`)
- [x] Add `opacity: f32` and `fading_out: bool` fields to `Surface` struct in engine.rs
- [x] Add `FadeCompleteEvent` struct and `PENDING_FADE_COMPLETIONS` thread-local to `crates/winpane-core/src/window.rs`
- [x] Add `WM_TIMER` handler to `hud_wndproc` in `crates/winpane-core/src/window.rs`
- [x] Add `WM_TIMER` handler to `panel_wndproc` in `crates/winpane-core/src/window.rs`
- [x] Add `FadeIn` and `FadeOut` handlers in engine command dispatch in `crates/winpane-core/src/engine.rs`
- [x] Add `process_fade_completions()` function in engine.rs and call it in main loop
- [x] Add opacity reset in `Show` command handler for previously faded-out surfaces
- [x] Add `fade_in()` and `fade_out()` to `Hud`, `Panel`, `Pip` in `crates/winpane/src/lib.rs`
- [x] Add `FfiSurface::fade_in/fade_out` and extern functions in `crates/winpane-ffi/src/lib.rs`
- [x] Add `fade_in`/`fade_out` to dispatch in `crates/winpane-host/src/dispatch.rs`
- [x] Add `fade_in`/`fade_out` to `SurfaceHandle` and `WinPane` in `bindings/node/src/lib.rs`
- [x] Run `cargo fmt --all` and verify `cargo fmt --all -- --check` passes
- [x] Mark phase complete

## Implementation Summary

Added DirectComposition-based fade animations across the full stack. The
`animate_opacity()` method on `SurfaceRenderer` creates an `IDCompositionAnimation`
with a linear cubic segment, attaches it to an `IDCompositionEffectGroup`, and
commits. For fade_out, a `WM_TIMER` one-shot fires after the animation duration
to hide the window, using the same thread-local queue pattern as DPI/tray/monitor
events. The `Show` command resets opacity when a surface was previously faded out.
