# P3: C ABI & FFI - Proposal

## Background

winpane is a Rust SDK for creating companion UI surfaces on Windows using DirectComposition. P0 bootstrapped the workspace. P1 built the HUD overlay with a retained-mode scene graph (text, rect, image elements keyed by string), Direct2D + DirectComposition rendering, and per-monitor DPI. P2 added interactive panels (selective click-through via hit-testing, mouse events), system tray icons (HICON from RGBA, popup panels, context menus), and polling-based event delivery.

The full Rust API surface: `Context` (internal SDK thread with message loop and lock-free command queue), `Hud` (click-through overlay), `Panel` (interactive surface), `Tray` (system tray icon), element operations (set_text, set_rect, set_image, remove), surface control (show, hide, set_position, set_size, set_opacity), and `poll_event()`.

This phase wraps all of that in a C ABI (`winpane-ffi` crate producing `winpane.dll`), making winpane consumable from C, C++, Go, Zig, C#, Python, and any language with C FFI support. It also adds a custom draw escape hatch for rendering beyond the declarative retained-mode primitives.

### Decisions from brainstorm

| Question | Decision | Rationale |
|----------|----------|-----------|
| Custom draw API surface | **A. Thin wrapper** | Safe C functions for rect, text, line, ellipse, image, rounded rect (10 drawing functions). Doesn't expose Direct2D types, keeps the header portable and manageable. Sufficient for charts, custom HUDs, and visualizations. If raw D2D access is ever needed, it can be added behind a separate opt-in function later without breaking the existing API. |
| Struct versioning | **A. Version + size fields** | Config structs start with `uint32_t version; uint32_t size;`. SDK reads only the fields it knows about based on the version number; new fields go at the end. This is the standard COM-era pattern (DirectX, Win32 OPENFILENAME). Well-understood by C consumers, zero allocation overhead, and forward-compatible without additional API calls. |

## Architecture

### Handle model

The FFI layer uses opaque pointer handles for all resource types. Consumers never see struct internals. Every handle has paired create/destroy functions, and the caller owns the lifecycle.

```
C consumer                          winpane-ffi                    winpane (Rust)
-----------                         -----------                    --------------
winpane_create(&ctx)           -->  Box::into_raw(Context::new())
winpane_hud_create(ctx, &cfg)  -->  ctx.create_hud(cfg) -> Box::into_raw(FfiSurface::Hud)
winpane_surface_set_text(s,..) -->  s.set_text(key, elem)
winpane_surface_destroy(s)     -->  Box::from_raw(s) -> drop
winpane_destroy(ctx)           -->  Box::from_raw(ctx) -> drop
```

Four opaque handle types:
- `winpane_context_t*` - wraps `winpane::Context`
- `winpane_surface_t*` - wraps either `winpane::Hud` or `winpane::Panel` (unified, see below)
- `winpane_tray_t*` - wraps `winpane::Tray`
- `winpane_canvas_t*` - temporary draw accumulator, valid between begin/end_draw

### Unified surface handle

Hud and Panel have identical element/surface operations (set_text, set_rect, show, hide, etc.). The only difference is creation config and event generation. Rather than duplicating every surface function for both types, the FFI layer uses a single `winpane_surface_t*` handle backed by an internal enum:

```rust
enum FfiSurface {
    Hud(winpane::Hud),
    Panel(winpane::Panel),
}
```

All `winpane_surface_*` functions dispatch through this enum. Creation is type-specific (`winpane_hud_create` vs `winpane_panel_create`), but everything after creation uses the unified surface handle. This halves the surface API function count without losing type information internally.

### Error handling

Every `extern "C"` function returns `int32_t`:
- `0` = success
- `-1` = error (details via `winpane_last_error()`)
- `-2` = panic caught at FFI boundary

`winpane_last_error()` returns a thread-local `const char*` with the error message. The pointer is valid until the next winpane call on the same thread. This is the standard pattern used by OpenSSL, SQLite, and most C libraries.

Implementation: a `ffi_try!` macro wraps every function body in `std::panic::catch_unwind`, converts `Result::Err` to thread-local error string, and converts panics to error code `-2`. Panicking across FFI is undefined behavior in Rust, so this is mandatory.

### Struct versioning

Config structs that cross the FFI boundary use version + size fields:

```c
#define WINPANE_CONFIG_VERSION 1

typedef struct {
    uint32_t version;    // WINPANE_CONFIG_VERSION
    uint32_t size;       // sizeof(this struct)
    // ... fields ...
} winpane_hud_config_t;
```

The FFI layer checks `version` and `size` before reading fields. If `size` is smaller than expected (old consumer, new SDK), fields beyond the consumer's struct size get defaults. New fields are always appended at the end.

Value-only structs (Color, elements, MenuItem, Event) do NOT use versioning. They are considered frozen per major version. Adding fields to these requires a new struct type (e.g., `winpane_text_element_v2_t`).

### Custom draw pipeline

The custom draw escape hatch lets consumers render arbitrary content beyond the retained-mode scene graph. It works via a buffered command list pattern (not direct D2D access):

```
C consumer                           winpane-ffi               Engine thread
-----------                          -----------               -------------
winpane_surface_begin_draw(s, &c)    create CanvasAccumulator
winpane_canvas_fill_rect(c, ...)     push DrawOp::FillRect
winpane_canvas_draw_text(c, ...)     push DrawOp::DrawText
winpane_surface_end_draw(s)          send Command::CustomDraw   execute ops on D2D target
                                     drop CanvasAccumulator     present
```

`begin_draw` creates a thread-local accumulator. Canvas calls push draw operations to a `Vec<DrawOp>`. `end_draw` sends the accumulated ops to the engine thread as a single command. The engine executes them on the surface's Direct2D render target and presents the result.

This design is thread-safe (no cross-thread D2D access), works with the existing command queue architecture, and doesn't leak D2D types into the public API. Custom draw renders on top of the retained-mode scene graph; call `winpane_canvas_clear()` first for a blank slate.

Constraint: custom draw only works in-process (same address space as the DLL). It will not be available over the IPC/stdio protocol in P5.

### cbindgen header generation

`cbindgen` (Mozilla's C/C++ header generator) runs via `build.rs` in the `winpane-ffi` crate. It reads `#[repr(C)]` structs and `extern "C"` functions and outputs `include/winpane.h`. Opaque handle types (non-`#[repr(C)]`) become forward-declared incomplete types in the header.

Configuration (`cbindgen.toml`):
- Language: C
- Header guard: `WINPANE_H`
- Style: type-prefixed with `winpane_` naming
- Include guard for C++ (`extern "C"` block)
- Exclude internal types (only annotated public types exported)

## Public API

### Types

```c
// --- Version ---
#define WINPANE_CONFIG_VERSION 1

// --- Opaque handles ---
typedef struct WinpaneContext winpane_context_t;
typedef struct WinpaneSurface winpane_surface_t;
typedef struct WinpaneTray winpane_tray_t;
typedef struct WinpaneCanvas winpane_canvas_t;

// --- Color ---
typedef struct {
    uint8_t r, g, b, a;
} winpane_color_t;

// --- Config structs (versioned) ---
typedef struct {
    uint32_t version;
    uint32_t size;
    int32_t x;
    int32_t y;
    uint32_t width;
    uint32_t height;
} winpane_hud_config_t;

typedef struct {
    uint32_t version;
    uint32_t size;
    int32_t x;
    int32_t y;
    uint32_t width;
    uint32_t height;
    int32_t draggable;       // 0 = false, nonzero = true
    uint32_t drag_height;    // logical pixels from top edge
} winpane_panel_config_t;

typedef struct {
    uint32_t version;
    uint32_t size;
    const uint8_t* icon_rgba;    // RGBA8 pixel data, borrowed
    uint32_t icon_rgba_len;      // byte length of icon_rgba
    uint32_t icon_width;
    uint32_t icon_height;
    const char* tooltip;         // UTF-8, null-terminated, max 127 chars
} winpane_tray_config_t;

// --- Element structs (value types, no versioning) ---
typedef struct {
    const char* text;            // UTF-8, null-terminated, borrowed
    float x;
    float y;
    float font_size;
    winpane_color_t color;
    const char* font_family;     // NULL for system default
    int32_t bold;
    int32_t italic;
    int32_t interactive;
} winpane_text_element_t;

typedef struct {
    float x;
    float y;
    float width;
    float height;
    winpane_color_t fill;
    float corner_radius;
    int32_t has_border;          // 0 = no border
    winpane_color_t border_color;
    float border_width;
    int32_t interactive;
} winpane_rect_element_t;

typedef struct {
    float x;
    float y;
    float width;
    float height;
    const uint8_t* data;         // RGBA8 premultiplied, row-major, borrowed
    uint32_t data_len;
    uint32_t data_width;
    uint32_t data_height;
    int32_t interactive;
} winpane_image_element_t;

// --- Menu item ---
typedef struct {
    uint32_t id;
    const char* label;           // UTF-8, null-terminated, borrowed
    int32_t enabled;             // 0 = disabled (grayed), nonzero = enabled
} winpane_menu_item_t;

// --- Event ---
typedef enum {
    WINPANE_EVENT_NONE = 0,
    WINPANE_EVENT_ELEMENT_CLICKED = 1,
    WINPANE_EVENT_ELEMENT_HOVERED = 2,
    WINPANE_EVENT_ELEMENT_LEFT = 3,
    WINPANE_EVENT_TRAY_CLICKED = 4,
    WINPANE_EVENT_TRAY_MENU_ITEM_CLICKED = 5,
} winpane_event_type_t;

typedef enum {
    WINPANE_MOUSE_LEFT = 0,
    WINPANE_MOUSE_RIGHT = 1,
    WINPANE_MOUSE_MIDDLE = 2,
} winpane_mouse_button_t;

typedef struct {
    winpane_event_type_t type;
    uint64_t surface_id;         // valid for ELEMENT_* events
    char key[256];               // element key, null-terminated; valid for ELEMENT_* events
    winpane_mouse_button_t mouse_button;  // valid for TRAY_CLICKED
    uint32_t menu_item_id;       // valid for TRAY_MENU_ITEM_CLICKED
} winpane_event_t;
```

### Functions

```c
// --- Error ---
const char* winpane_last_error(void);

// --- Context lifecycle ---
int32_t winpane_create(winpane_context_t** out);
void    winpane_destroy(winpane_context_t* ctx);

// --- Surface creation (type-specific) ---
int32_t winpane_hud_create(winpane_context_t* ctx,
                           const winpane_hud_config_t* config,
                           winpane_surface_t** out);
int32_t winpane_panel_create(winpane_context_t* ctx,
                             const winpane_panel_config_t* config,
                             winpane_surface_t** out);

// --- Surface operations (unified) ---
void     winpane_surface_destroy(winpane_surface_t* surface);
uint64_t winpane_surface_id(const winpane_surface_t* surface);
int32_t  winpane_surface_set_text(winpane_surface_t* surface,
                                  const char* key,
                                  const winpane_text_element_t* element);
int32_t  winpane_surface_set_rect(winpane_surface_t* surface,
                                  const char* key,
                                  const winpane_rect_element_t* element);
int32_t  winpane_surface_set_image(winpane_surface_t* surface,
                                   const char* key,
                                   const winpane_image_element_t* element);
int32_t  winpane_surface_remove(winpane_surface_t* surface, const char* key);
int32_t  winpane_surface_show(winpane_surface_t* surface);
int32_t  winpane_surface_hide(winpane_surface_t* surface);
int32_t  winpane_surface_set_position(winpane_surface_t* surface, int32_t x, int32_t y);
int32_t  winpane_surface_set_size(winpane_surface_t* surface, uint32_t width, uint32_t height);
int32_t  winpane_surface_set_opacity(winpane_surface_t* surface, float opacity);

// --- Tray ---
int32_t winpane_tray_create(winpane_context_t* ctx,
                            const winpane_tray_config_t* config,
                            winpane_tray_t** out);
void    winpane_tray_destroy(winpane_tray_t* tray);
int32_t winpane_tray_set_tooltip(winpane_tray_t* tray, const char* tooltip);
int32_t winpane_tray_set_icon(winpane_tray_t* tray,
                              const uint8_t* rgba, uint32_t rgba_len,
                              uint32_t width, uint32_t height);
int32_t winpane_tray_set_popup(winpane_tray_t* tray, const winpane_surface_t* panel);
int32_t winpane_tray_set_menu(winpane_tray_t* tray,
                              const winpane_menu_item_t* items,
                              uint32_t count);

// --- Events ---
int32_t winpane_poll_event(winpane_context_t* ctx, winpane_event_t* event);

// --- Custom draw ---
int32_t winpane_surface_begin_draw(winpane_surface_t* surface,
                                   winpane_canvas_t** out);
int32_t winpane_surface_end_draw(winpane_surface_t* surface);
int32_t winpane_canvas_clear(winpane_canvas_t* canvas, winpane_color_t color);
int32_t winpane_canvas_fill_rect(winpane_canvas_t* canvas,
                                 float x, float y, float w, float h,
                                 winpane_color_t color);
int32_t winpane_canvas_stroke_rect(winpane_canvas_t* canvas,
                                   float x, float y, float w, float h,
                                   winpane_color_t color, float width);
int32_t winpane_canvas_draw_text(winpane_canvas_t* canvas,
                                 float x, float y, const char* text,
                                 float font_size, winpane_color_t color);
int32_t winpane_canvas_draw_line(winpane_canvas_t* canvas,
                                 float x1, float y1, float x2, float y2,
                                 winpane_color_t color, float width);
int32_t winpane_canvas_fill_ellipse(winpane_canvas_t* canvas,
                                    float cx, float cy, float rx, float ry,
                                    winpane_color_t color);
int32_t winpane_canvas_stroke_ellipse(winpane_canvas_t* canvas,
                                      float cx, float cy, float rx, float ry,
                                      winpane_color_t color, float width);
int32_t winpane_canvas_draw_image(winpane_canvas_t* canvas,
                                  float x, float y, float w, float h,
                                  const uint8_t* rgba, uint32_t rgba_len,
                                  uint32_t img_w, uint32_t img_h);
int32_t winpane_canvas_fill_rounded_rect(winpane_canvas_t* canvas,
                                         float x, float y, float w, float h,
                                         float radius, winpane_color_t color);
int32_t winpane_canvas_stroke_rounded_rect(winpane_canvas_t* canvas,
                                           float x, float y, float w, float h,
                                           float radius, winpane_color_t color,
                                           float width);
```

**Function count:** 35 total (1 error, 2 context, 2 surface creation, 11 surface ops, 6 tray, 1 event, 2 draw lifecycle, 10 canvas drawing).

### Usage example

```c
#include "winpane.h"

int main(void) {
    winpane_context_t* ctx = NULL;
    if (winpane_create(&ctx) != 0) {
        printf("Error: %s\n", winpane_last_error());
        return 1;
    }

    winpane_hud_config_t cfg = {
        .version = WINPANE_CONFIG_VERSION,
        .size = sizeof(winpane_hud_config_t),
        .x = 100, .y = 100,
        .width = 400, .height = 200,
    };
    winpane_surface_t* hud = NULL;
    winpane_hud_create(ctx, &cfg, &hud);

    winpane_text_element_t text = {
        .text = "Hello from C!",
        .x = 20.0f, .y = 20.0f,
        .font_size = 24.0f,
        .color = { 255, 255, 255, 255 },
    };
    winpane_surface_set_text(hud, "greeting", &text);
    winpane_surface_show(hud);

    // Event loop
    winpane_event_t event;
    while (1) {
        if (winpane_poll_event(ctx, &event) == 0) {
            if (event.type == WINPANE_EVENT_ELEMENT_CLICKED) {
                printf("Clicked: %s\n", event.key);
            }
        }
        Sleep(16);
    }

    winpane_surface_destroy(hud);
    winpane_destroy(ctx);
    return 0;
}
```

## Plan

### 1. Custom draw support in winpane-core

Add `DrawOp` enum to `winpane-core/src/types.rs`: `Clear`, `FillRect`, `StrokeRect`, `DrawText`, `DrawLine`, `FillEllipse`, `StrokeEllipse`, `DrawImage`, `FillRoundedRect`, `StrokeRoundedRect`. Add `Command::CustomDraw { surface: SurfaceId, ops: Vec<DrawOp> }` to `command.rs`. Handle the command in the engine loop by executing the draw ops on the surface's Direct2D render target. Add `SurfaceRenderer::execute_draw_ops(&mut self, ops: &[DrawOp])` to the render module.

**Files:** `crates/winpane-core/src/types.rs` (modify), `crates/winpane-core/src/command.rs` (modify), `crates/winpane-core/src/engine.rs` (modify), `crates/winpane-core/src/render.rs` (modify)

### 2. Custom draw in winpane public API

Expose `custom_draw(ops: Vec<DrawOp>)` on `Hud` and `Panel` in the public crate. Re-export `DrawOp` from `winpane`. This sends `Command::CustomDraw` via the command queue and wakes the engine, following the same fire-and-forget pattern as all other surface operations.

**Files:** `crates/winpane/src/lib.rs` (modify)

### 3. FFI crate setup

Update `winpane-ffi/Cargo.toml`: add `winpane` dependency (path), `libc` for C types, set `crate-type = ["cdylib"]`. Add `cbindgen` as a build dependency. Create `cbindgen.toml` with C language, `WINPANE_H` header guard, `winpane_` prefix, C++ extern block. Create `build.rs` that runs cbindgen to generate `include/winpane.h`. Create `crates/winpane-ffi/include/` directory.

**Files:** `crates/winpane-ffi/Cargo.toml` (modify), `crates/winpane-ffi/cbindgen.toml` (create), `crates/winpane-ffi/build.rs` (create)

### 4. FFI error handling infrastructure

Implement thread-local error storage using `thread_local!` with `RefCell<Option<CString>>`. Implement `set_last_error(impl Display)` and `set_last_error_str(&str)` helpers. Create `ffi_try!` macro that wraps function bodies in `catch_unwind`, maps `Result::Err` to error code -1 with `set_last_error`, and maps panics to error code -2. Implement `winpane_last_error()`.

**Files:** `crates/winpane-ffi/src/lib.rs` (modify)

### 5. C-compatible type definitions

Define all `#[repr(C)]` structs in the FFI crate: `WinpaneColor`, config structs (with version/size), element structs, `WinpaneMenuItem`, `WinpaneEventType` enum, `WinpaneMouseButton` enum, `WinpaneEvent` struct. Implement conversion functions from C types to Rust types (e.g., `WinpaneTextElement -> TextElement`, `WinpaneHudConfig -> HudConfig`). Null pointer validation on all borrowed pointers.

**Files:** `crates/winpane-ffi/src/lib.rs` (modify), or split into `src/types.rs`

### 6. Opaque handle types and context functions

Define internal `FfiSurface` enum, `WinpaneContext`, `WinpaneSurface`, `WinpaneTray`, `WinpaneCanvas` structs (not `#[repr(C)]`, so cbindgen generates opaque forward declarations). Implement `winpane_create` (calls `Context::new()`, boxes result), `winpane_destroy` (calls `Box::from_raw`, drops), `winpane_poll_event` (calls `ctx.poll_event()`, converts Event to WinpaneEvent).

For `poll_event`: returns 0 and fills the event struct if an event is available, returns 1 if no event (WINPANE_EVENT_NONE).

**Files:** `crates/winpane-ffi/src/lib.rs` (modify)

### 7. Surface creation and operations

Implement `winpane_hud_create` (validates config version/size, converts to HudConfig, calls `ctx.create_hud()`, wraps in `FfiSurface::Hud`). Same pattern for `winpane_panel_create`. Implement all `winpane_surface_*` functions: each validates the handle, converts C types to Rust types, dispatches through the FfiSurface enum to the appropriate Hud/Panel method. Implement `winpane_surface_destroy` and `winpane_surface_id`.

`winpane_surface_destroy` uses `Box::from_raw` to reclaim and drop the surface. The Rust Drop impl sends the destroy command to the engine.

**Files:** `crates/winpane-ffi/src/lib.rs` (modify)

### 8. Tray functions

Implement `winpane_tray_create` (converts TrayConfig, calls `ctx.create_tray()`, boxes result), `winpane_tray_destroy`, `winpane_tray_set_tooltip`, `winpane_tray_set_icon`, `winpane_tray_set_menu`. For `winpane_tray_set_popup`: extracts the SurfaceId from the surface handle and sends `SetTrayPopup` command directly (since the Rust API takes `&Panel` but the C API has unified surface handles). Returns an error if the surface is not a Panel.

**Files:** `crates/winpane-ffi/src/lib.rs` (modify)

### 9. Canvas functions (custom draw FFI)

Implement `winpane_surface_begin_draw` (creates a `WinpaneCanvas` containing the surface handle reference and an empty `Vec<DrawOp>`). Each `winpane_canvas_*` function converts C arguments to a `DrawOp` and pushes to the vec. `winpane_surface_end_draw` takes the accumulated ops, calls `surface.custom_draw(ops)`, and drops the canvas.

The canvas pointer is stored alongside the surface handle (one active canvas per surface at a time). `begin_draw` returns an error if a canvas is already active. `end_draw` returns an error if no canvas is active.

**Files:** `crates/winpane-ffi/src/lib.rs` (modify)

### 10. C header verification and .def file

Verify that `include/winpane.h` compiles cleanly with MSVC (`cl /c /W4`), Clang (`clang -fsyntax-only -Wall`), and MinGW (`gcc -fsyntax-only -Wall`). Fix any warnings. Generate `include/winpane.def` listing all exported symbols for consumers who need a module definition file.

**Verification in CI:** add a step that compiles a minimal C file including `winpane.h`.

**Files:** `crates/winpane-ffi/include/winpane.def` (create), `.github/workflows/ci.yml` (modify)

### 11. C examples

**`examples/c/hello_hud.c`** - create a context, create a HUD, add text and rect elements, show the surface, poll events in a loop, clean up. Compile instructions in comments.

**`examples/c/custom_draw.c`** - create a context, create a HUD, use begin_draw/canvas_*/end_draw to render a custom visualization (colored rectangles, text labels, a simple bar chart), clean up.

**`examples/c/CMakeLists.txt`** - CMake build script that finds `winpane.dll` + `winpane.lib` + `winpane.h` and builds both examples. Also include a simple batch file (`build.bat`) for consumers who just want `cl hello_hud.c /I... /link winpane.lib`.

**Files:** `examples/c/hello_hud.c` (create), `examples/c/custom_draw.c` (create), `examples/c/CMakeLists.txt` (create), `examples/c/build.bat` (create)

### 12. Rust custom draw example

**`examples/rust/custom_draw.rs`** - demonstrates the Rust-side `custom_draw()` API added in step 2. Creates a surface, draws a gradient-like pattern with filled rects, text overlay, and an ellipse. Verifies the DrawOp pipeline works end-to-end from Rust before the FFI layer depends on it.

**Files:** `examples/rust/custom_draw.rs` (create)

## File manifest

| File | Action | Purpose |
|------|--------|---------|
| `crates/winpane-core/src/types.rs` | Modify | Add `DrawOp` enum |
| `crates/winpane-core/src/command.rs` | Modify | Add `Command::CustomDraw` variant |
| `crates/winpane-core/src/engine.rs` | Modify | Handle `CustomDraw` command |
| `crates/winpane-core/src/render.rs` | Modify | Add `execute_draw_ops` to `SurfaceRenderer` |
| `crates/winpane-core/src/lib.rs` | Modify | Re-export `DrawOp` |
| `crates/winpane/src/lib.rs` | Modify | Add `custom_draw()` to Hud/Panel, re-export `DrawOp` |
| `crates/winpane-ffi/Cargo.toml` | Modify | Add dependencies, set crate-type = ["cdylib"] |
| `crates/winpane-ffi/cbindgen.toml` | Create | cbindgen configuration |
| `crates/winpane-ffi/build.rs` | Create | Header generation via cbindgen |
| `crates/winpane-ffi/src/lib.rs` | Modify | All FFI functions, types, error handling, canvas |
| `crates/winpane-ffi/include/winpane.h` | Generated | Auto-generated C header |
| `crates/winpane-ffi/include/winpane.def` | Create | Module definition file for DLL exports |
| `examples/c/hello_hud.c` | Create | C HUD overlay example |
| `examples/c/custom_draw.c` | Create | C custom draw example |
| `examples/c/CMakeLists.txt` | Create | CMake build for C examples |
| `examples/c/build.bat` | Create | Simple MSVC build script |
| `examples/rust/custom_draw.rs` | Create | Rust custom draw example |
| `.github/workflows/ci.yml` | Modify | Add header compilation check |

## Dependencies

### New crate dependencies

**winpane-ffi/Cargo.toml:**
- `winpane` (path = "../winpane") - the public Rust API to wrap
- `libc` - C-compatible types (`c_char`, `c_int`)

**winpane-ffi build dependency:**
- `cbindgen` - C/C++ header generation from Rust source

### No changes to winpane-core or winpane dependencies

The `DrawOp` types use only existing `Color` and `String` types already in winpane-core.

## Out of scope

- Async/callback event delivery (polling only, matching the Rust API)
- C++ class wrappers or RAII helpers (consumers write their own or use a future `winpane.hpp`)
- Go/Python/C# bindings (those wrap the C ABI; this phase produces the C ABI they'll use)
- Custom draw with text layout control (font family, bold, italic in canvas ops); retained-mode TextElement covers styled text
- Streaming/incremental draw (canvas ops are batched per end_draw call)
- Surface resize during custom draw (end_draw must complete before set_size takes effect)
- winpane.pc (pkg-config file); can be added in P6
- NuGet/vcpkg/Conan packaging; can be added in P6

## Verification

P3 is complete when:

1. `cargo build --workspace` passes on CI (windows-latest), producing `winpane.dll` + `winpane.lib`
2. `cargo clippy --workspace -- -D warnings` passes
3. `cargo test --workspace` passes (existing P1/P2 tests still pass)
4. `include/winpane.h` is auto-generated and compiles without warnings on MSVC (`cl /c /W4`), Clang, and MinGW
5. `examples/c/hello_hud.c` compiles with MSVC, links against `winpane.lib`, runs and displays a HUD overlay with text
6. `examples/c/custom_draw.c` compiles, links, runs and displays custom-drawn content (rects, text, ellipse)
7. `examples/rust/custom_draw.rs` runs and displays custom-drawn content via the Rust API
8. `winpane_last_error()` returns meaningful error messages (test by passing NULL pointers or invalid configs)
9. All 35 `extern "C"` functions are present in `winpane.dll` exports (verify with `dumpbin /exports winpane.dll`)
10. No panics escape the FFI boundary (verified by catch_unwind on every function)
11. `cargo fmt --all -- --check` passes (pre-push check)
