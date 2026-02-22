# Phase 4: FFI Functions - Surface, Tray, and Canvas

## Overview

Implement all remaining FFI functions: surface creation (2), surface operations (11), tray functions (6), and canvas/custom draw functions (12). This is the bulk of the FFI API - 31 functions total. All functions follow the same pattern: validate pointers, convert C types to Rust, dispatch through opaque handles, return status code via `ffi_try!`.

## Prerequisites

- Phase 3 complete (all types, opaque handles, context lifecycle, event polling in lib.rs)
- Read `initial-plan.md` Phases 7-9
- Read `proposal.md` "Unified surface handle" and "Custom draw pipeline" sections

## What to Build

All code goes in `crates/winpane-ffi/src/lib.rs`, appended after the Phase 3 code.

### 1. Surface creation (2 functions)

**`winpane_hud_create(ctx, config, out) -> i32`**
- Validates all 3 pointers
- Converts `WinpaneHudConfig` via `to_rust()`
- Calls `ctx.inner.create_hud(cfg)`
- Wraps result in `WinpaneSurface { inner: FfiSurface::Hud(hud), canvas: None }`
- Boxes and writes to out pointer

**`winpane_panel_create(ctx, config, out) -> i32`**
- Same pattern as hud_create but with `WinpanePanelConfig` and `FfiSurface::Panel`

### 2. Surface operations (11 functions)

**`winpane_surface_destroy(surface: *mut WinpaneSurface)`**
- Returns void. If non-null: `Box::from_raw` to reclaim and drop
- The Rust Drop impl on Hud/Panel sends the destroy command to the engine

**`winpane_surface_id(surface: *const WinpaneSurface) -> u64`**
- Returns 0 if null. Otherwise returns `surface.inner.id().0`

**`winpane_surface_set_text(surface, key, element) -> i32`**
- Validates all 3 pointers
- Converts key via `cstr_to_string`, element via `to_rust()`
- Calls `surface.inner.set_text(&key, elem)`

**`winpane_surface_set_rect(surface, key, element) -> i32`**
- Same pattern, converts `WinpaneRectElement`

**`winpane_surface_set_image(surface, key, element) -> i32`**
- Same pattern, converts `WinpaneImageElement`

**`winpane_surface_remove(surface, key) -> i32`**
- Validates both pointers, converts key, calls `surface.inner.remove(&key)`

**`winpane_surface_show(surface) -> i32`**
**`winpane_surface_hide(surface) -> i32`**
- Validate pointer, call `surface.inner.show()` / `hide()`

**`winpane_surface_set_position(surface, x, y) -> i32`**
**`winpane_surface_set_size(surface, width, height) -> i32`**
**`winpane_surface_set_opacity(surface, opacity) -> i32`**
- Validate pointer, call the corresponding method on `surface.inner`

### 3. Tray functions (6 functions)

**`winpane_tray_create(ctx, config, out) -> i32`**
- Validates pointers, converts `WinpaneTrayConfig` (unsafe - borrows icon data, tooltip string)
- Calls `ctx.inner.create_tray(cfg)`, boxes as `WinpaneTray`

**`winpane_tray_destroy(tray: *mut WinpaneTray)`**
- Void return. Box::from_raw if non-null.

**`winpane_tray_set_tooltip(tray, tooltip) -> i32`**
- Converts tooltip string, calls `tray.inner.set_tooltip(&text)`

**`winpane_tray_set_icon(tray, rgba, rgba_len, width, height) -> i32`**
- Validates pointers, copies RGBA slice, calls `tray.inner.set_icon(data, width, height)`

**`winpane_tray_set_popup(tray, panel) -> i32`**
- Validates both pointers
- Extracts Panel from `FfiSurface` enum. Returns error if surface is a Hud.
- Calls `tray.inner.set_popup(panel_ref)`

**`winpane_tray_set_menu(tray, items, count) -> i32`**
- If count > 0, validates items pointer
- Iterates items array, converts each `WinpaneMenuItem` (label via `cstr_to_string`, enabled via i32 != 0)
- Calls `tray.inner.set_menu(menu_items)`

### 4. Canvas/custom draw functions (12 functions)

**`winpane_surface_begin_draw(surface, out) -> i32`**
- Validates pointers
- Checks `surface.canvas.is_none()` (error if already active)
- Creates `CanvasAccumulator` with empty ops vec
- Gets raw pointer to ops vec
- Stores accumulator in `surface.canvas`
- Creates `WinpaneCanvas { ops: ops_ptr }`, boxes, writes to out

**`winpane_surface_end_draw(surface) -> i32`**
- Validates pointer
- Takes `surface.canvas` via `.take()` (error if None)
- Calls `surface.inner.custom_draw(acc.ops)`
- The canvas handle becomes dangling after this call - any use is undefined behavior
- Document this clearly in the C header comment for `winpane_surface_end_draw`: "The canvas handle is invalid after this call. Do not use it."

**10 canvas drawing functions** - each validates the canvas pointer, then pushes a `DrawOp` to the ops vec:

| Function | DrawOp | Parameters |
|----------|--------|------------|
| `winpane_canvas_clear` | `Clear(color)` | color |
| `winpane_canvas_fill_rect` | `FillRect` | x, y, w, h, color |
| `winpane_canvas_stroke_rect` | `StrokeRect` | x, y, w, h, color, width |
| `winpane_canvas_draw_text` | `DrawText` | x, y, text, font_size, color |
| `winpane_canvas_draw_line` | `DrawLine` | x1, y1, x2, y2, color, width |
| `winpane_canvas_fill_ellipse` | `FillEllipse` | cx, cy, rx, ry, color |
| `winpane_canvas_stroke_ellipse` | `StrokeEllipse` | cx, cy, rx, ry, color, width |
| `winpane_canvas_draw_image` | `DrawImage` | x, y, w, h, rgba, rgba_len, img_w, img_h |
| `winpane_canvas_fill_rounded_rect` | `FillRoundedRect` | x, y, w, h, radius, color |
| `winpane_canvas_stroke_rounded_rect` | `StrokeRoundedRect` | x, y, w, h, radius, color, width |

All canvas functions use `ffi_try!`, validate the canvas pointer, then `unsafe { &mut *(*canvas).ops }` to access the ops vec.

`winpane_canvas_draw_text` additionally converts the text string via `cstr_to_string`.
`winpane_canvas_draw_image` additionally validates the rgba pointer and copies the slice.

See `initial-plan.md` Phases 7-9 for the full implementation code.

## Connections

- **Previous phase:** Phase 3 (types, handles, context lifecycle defined)
- **Next phase:** Phase 5 (C examples and CI) validates all these functions work end-to-end

## Checkpoint

After this phase: `cargo build --workspace` succeeds. The full FFI layer is implemented with 35 total exported functions (1 error + 2 context + 1 event + 2 creation + 11 surface + 6 tray + 12 canvas). The generated `winpane.h` should contain all function prototypes. Run `cargo fmt --all` and `cargo clippy --workspace` (on Windows CI).
