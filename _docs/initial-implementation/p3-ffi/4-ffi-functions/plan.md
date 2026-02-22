# Phase 4: FFI Functions - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p3-ffi/learnings.md`
2. `_docs/initial-implementation/p3-ffi/initial-plan.md` Phases 7-9
3. `_docs/initial-implementation/p3-ffi/4-ffi-functions/spec.md`
4. `crates/winpane-ffi/src/lib.rs` (current state after Phase 3)
5. `crates/winpane/src/lib.rs` (for Rust API method signatures)

## Implementation Checklist

- [ ] Implement `winpane_hud_create()` and `winpane_panel_create()`
- [ ] Implement `winpane_surface_destroy()` and `winpane_surface_id()`
- [ ] Implement `winpane_surface_set_text()`, `winpane_surface_set_rect()`, `winpane_surface_set_image()`
- [ ] Implement `winpane_surface_remove()`, `winpane_surface_show()`, `winpane_surface_hide()`
- [ ] Implement `winpane_surface_set_position()`, `winpane_surface_set_size()`, `winpane_surface_set_opacity()`
- [ ] Implement `winpane_tray_create()` and `winpane_tray_destroy()`
- [ ] Implement `winpane_tray_set_tooltip()`, `winpane_tray_set_icon()`, `winpane_tray_set_popup()`, `winpane_tray_set_menu()`
- [ ] Implement `winpane_surface_begin_draw()` and `winpane_surface_end_draw()`
- [ ] Implement all 10 canvas drawing functions (clear, fill_rect, stroke_rect, draw_text, draw_line, fill_ellipse, stroke_ellipse, draw_image, fill_rounded_rect, stroke_rounded_rect)
- [ ] Verify `cargo build --workspace` succeeds and `winpane.h` contains all 35 function prototypes
- [ ] Run `cargo fmt --all`
- [ ] Mark phase complete in root plan.md

## Implementation Summary

*(To be filled after implementation)*
