# Phase 3: FFI Types and Opaque Handles

## Overview

Define all `#[repr(C)]` types that cross the FFI boundary (config structs, element structs, event types), their conversion functions to Rust types, and the opaque handle types (context, surface, tray, canvas). Implement context lifecycle (`winpane_create`, `winpane_destroy`) and event polling (`winpane_poll_event`).

## Prerequisites

- Phase 2 complete (error handling infrastructure, ffi_try! macro, helpers in lib.rs)
- Read `initial-plan.md` Phases 5-6
- Read `proposal.md` "Struct versioning" and "Handle model" sections

## What to Build

All code goes in `crates/winpane-ffi/src/lib.rs`, appended after the Phase 2 error handling code.

### 1. Config version constant

```rust
pub const WINPANE_CONFIG_VERSION: u32 = 1;
```

### 2. WinpaneColor

`#[repr(C)]` struct with `r, g, b, a: u8`. Derives `Debug, Clone, Copy`.
Conversion: `fn to_rust(&self) -> winpane::Color` using `Color::rgba()`.

### 3. Versioned config structs

Each starts with `version: u32, size: u32`. The `to_rust()` method checks:
1. `version == WINPANE_CONFIG_VERSION` - returns `Err` on mismatch
2. `size >= std::mem::size_of::<Self>()` - catches obviously wrong sizes (e.g. consumer passed 0 or a truncated struct)

The size check is a safety net. Forward-compatibility logic (reading fewer fields from old consumers) is deferred until version 2 is needed.

**WinpaneHudConfig:** version, size, x (i32), y (i32), width (u32), height (u32)
- `to_rust() -> Result<winpane::HudConfig, String>`

**WinpanePanelConfig:** version, size, x, y, width, height, draggable (i32, 0=false), drag_height (u32)
- `to_rust() -> Result<winpane::PanelConfig, String>`

**WinpaneTrayConfig:** version, size, icon_rgba (*const u8), icon_rgba_len (u32), icon_width, icon_height, tooltip (*const c_char)
- `unsafe fn to_rust() -> Result<winpane::TrayConfig, String>` - validates non-null pointers, copies icon data, converts tooltip string

### 4. Element structs (value types, no versioning)

**WinpaneTextElement:** text (*const c_char), x, y, font_size (f32), color (WinpaneColor), font_family (*const c_char, NULL for default), bold/italic/interactive (i32)
- `unsafe fn to_rust() -> Result<winpane::TextElement, String>` - converts strings, maps i32 to bool

**WinpaneRectElement:** x, y, width, height (f32), fill (WinpaneColor), corner_radius, has_border (i32), border_color (WinpaneColor), border_width, interactive (i32)
- `fn to_rust() -> winpane::RectElement` - maps has_border to Option<Color>

**WinpaneImageElement:** x, y, width, height (f32), data (*const u8), data_len/data_width/data_height (u32), interactive (i32)
- `unsafe fn to_rust() -> Result<winpane::ImageElement, String>` - validates non-null, copies data

### 5. Menu item

**WinpaneMenuItem:** id (u32), label (*const c_char), enabled (i32)

No conversion method needed - converted inline in the tray_set_menu function (Phase 4).

### 6. Event types

**WinpaneEventType** `#[repr(C)]` enum: None=0, ElementClicked=1, ElementHovered=2, ElementLeft=3, TrayClicked=4, TrayMenuItemClicked=5

**WinpaneMouseButton** `#[repr(C)]` enum: Left=0, Right=1, Middle=2

**WinpaneEvent** `#[repr(C)]` struct:
- event_type: WinpaneEventType (note: the proposal draft header used `type`, but `type` is a C++ reserved keyword and breaks `cpp_compat = true`. Use `event_type` instead.)
- surface_id: u64
- key: [u8; 256] (null-terminated UTF-8; truncates at 255 bytes, which could split multi-byte UTF-8 sequences for very long keys - acceptable since element keys are short identifiers)
- mouse_button: WinpaneMouseButton
- menu_item_id: u32

Conversion: `fn from_rust(event: &winpane::Event) -> Self` - matches Event variants, copies key into fixed buffer via `copy_key_to_buffer()` helper.

**copy_key_to_buffer(key: &str, buf: &mut [u8; 256]):** copies min(key.len(), 255) bytes, null-terminates.

### 7. Opaque handle types

These are NOT `#[repr(C)]`, so cbindgen generates opaque forward declarations in the C header.

**FfiSurface enum** (internal):
```rust
enum FfiSurface {
    Hud(winpane::Hud),
    Panel(winpane::Panel),
}
```

With dispatch methods: `id()`, `set_text()`, `set_rect()`, `set_image()`, `remove()`, `show()`, `hide()`, `set_position()`, `set_size()`, `set_opacity()`, `custom_draw()`. Each matches on the enum and delegates.

**WinpaneContext:** wraps `winpane::Context`

**WinpaneSurface:** wraps `FfiSurface` + `canvas: Option<Box<CanvasAccumulator>>`

**WinpaneTray:** wraps `winpane::Tray`

**CanvasAccumulator** (internal): `ops: Vec<winpane::DrawOp>`

**WinpaneCanvas:** `ops: *mut Vec<winpane::DrawOp>` (raw pointer into the accumulator)

### 8. Context lifecycle functions

**`winpane_create(out: *mut *mut WinpaneContext) -> i32`**
- Validates out non-null
- Calls `winpane::Context::new()`, boxes, writes to out pointer
- Uses `ffi_try!`

**`winpane_destroy(ctx: *mut WinpaneContext)`**
- Returns void (no error possible)
- If non-null: `Box::from_raw(ctx)` to reclaim and drop

### 9. Event polling

**`winpane_poll_event(ctx: *mut WinpaneContext, event: *mut WinpaneEvent) -> i32`**
- Validates both pointers
- Calls `ctx.inner.poll_event()`
- If `Some(e)`: writes `WinpaneEvent::from_rust(&e)` to out pointer, returns 0
- If `None`: sets `event_type` to `WinpaneEventType::None`, returns **1** (no event pending)
- Returns -1/-2 on error/panic as usual
- Note: this function cannot use `ffi_try!` directly because the None branch returns 1 (not 0). Use `catch_unwind` manually, or use `ffi_try!` only for the error path and handle the None case before it. The initial-plan.md code returns 0 for both paths and must be updated to match this spec.

See `initial-plan.md` Phases 5-6 for the full implementation code of all types and functions.

## Connections

- **Previous phase:** Phase 2 (error handling infra, ffi_try! macro, helpers)
- **Next phase:** Phase 4 (FFI Functions) uses these types and handles to implement surface/tray/canvas functions

## Checkpoint

After this phase: `cargo build --workspace` succeeds. The generated `winpane.h` header contains all type definitions, enum declarations, opaque handle forward declarations, and the context/event function prototypes.
