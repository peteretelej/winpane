# P3: C ABI & FFI - Brainstorm

> **Before starting**: Read `phases-progress.md` in the parent directory for project context, key decisions, and what previous phases built.
> **After completing**: Update `phases-progress.md` with what was built, key files, API surface, and anything the next phase needs to know.

## Background

**winpane** is an MIT-licensed Rust SDK for creating companion UI surfaces on Windows. It uses out-of-process DirectComposition windows (no process injection) with a retained-mode API. The primary audience is developer tool authors building AI assistants, dev tools, and status bars.

Previous phases built the core Rust API: `Context` (internal SDK thread with message loop and lock-free command queue), `Hud` (click-through transparent overlay), `Panel` (interactive surface with mouse events), `Tray` (system tray icon with popup), retained-mode scene graph (text, rect, image elements referenced by string keys), event polling, and Direct2D + DirectComposition rendering with per-monitor DPI.

This phase exposes all of that to non-Rust languages via a C ABI, making winpane consumable from C, C++, Go, Zig, C#, Python, and any language with C FFI support.

### Technical context

**C ABI conventions**: The C ABI is the universal FFI lingua franca. Every language can call `extern "C"` functions. Key patterns:
- Opaque handle pointers (not `void*`, use typed `winpane_context_t*`)
- Paired create/destroy functions for every resource (caller owns lifecycle)
- Integer error codes with `winpane_last_error()` returning a thread-local `const char*`
- `std::panic::catch_unwind` wrapping every `extern "C"` function (panicking across FFI is undefined behavior in Rust)
- `#[repr(C)]` structs for configs, with version+size fields for forward compatibility
- All strings are `const char*` UTF-8 null-terminated

**cbindgen**: Mozilla's tool for auto-generating C/C++ headers from Rust source. Run via `build.rs` in CI to keep headers synchronized. Used by rav1e (AV1 encoder) and Firefox.

**Custom draw escape hatch**: Power users may want rendering beyond the declarative primitives (text, rect, image). The escape hatch provides raw drawing access: `winpane_surface_begin_draw()` starts a frame, the consumer draws via C functions (or directly to a D2D render target pointer), and `winpane_surface_end_draw()` commits it. This only works in-process (not over IPC).

## Goal

Create the `winpane-ffi` crate: a cdylib exposing the winpane API as `extern "C"` functions with auto-generated C headers. Also implement the custom draw escape hatch for power users.

## Scope

### winpane-ffi crate
- `cdylib` target producing `winpane.dll` + `winpane.lib`
- Complete coverage of the Rust API surface:
  - Context lifecycle: `winpane_create`, `winpane_destroy`
  - HUD: `winpane_hud_create`, `winpane_hud_destroy`
  - Panel: `winpane_panel_create`, `winpane_panel_destroy`
  - Tray: `winpane_tray_create`, `winpane_tray_destroy`
  - Elements: `winpane_surface_set_text`, `winpane_surface_set_rect`, `winpane_surface_set_image`, `winpane_surface_remove`
  - Surface control: `winpane_surface_show`, `winpane_surface_hide`, `winpane_surface_set_position`, `winpane_surface_set_opacity`
  - Events: `winpane_poll_event`
  - Errors: `winpane_last_error`

### C header generation
- `cbindgen.toml` config in `winpane-ffi/`
- `build.rs` runs cbindgen, outputs `winpane.h`
- Header included in release artifacts alongside `.dll` and `.lib`
- Verify header compiles with MSVC, Clang, and MinGW

### Config structs
```c
typedef struct {
    uint32_t version;    // Set to WINPANE_CONFIG_VERSION
    uint32_t size;       // sizeof(winpane_config_t)
    // ... fields
} winpane_config_t;
```

### Custom draw escape hatch
```c
winpane_canvas_t* canvas = NULL;
int32_t err = winpane_surface_begin_draw(surface, &canvas);
if (err == 0) {
    winpane_canvas_draw_rect(canvas, x, y, w, h, color);
    winpane_canvas_draw_text(canvas, x, y, "Hello", font_size, color);
    winpane_canvas_draw_line(canvas, x1, y1, x2, y2, color, thickness);
    winpane_canvas_draw_ellipse(canvas, cx, cy, rx, ry, color);
    winpane_canvas_draw_image(canvas, x, y, w, h, png_bytes, png_len);
    winpane_surface_end_draw(surface);
}
```

### Examples
- `examples/c/hello_hud.c` - create a HUD overlay from C, compile with MSVC
- `examples/c/custom_draw.c` - use the custom draw escape hatch
- `examples/c/CMakeLists.txt` or build script showing how to link

## Deliverables

- [ ] `winpane-ffi` crate builds as cdylib
- [ ] Auto-generated `winpane.h` via cbindgen
- [ ] All Rust API surface types exposed via C functions
- [ ] `std::panic::catch_unwind` on every `extern "C"` boundary
- [ ] Thread-local error reporting via `winpane_last_error()`
- [ ] Custom draw API: begin/end draw + canvas drawing functions
- [ ] `examples/c/hello_hud.c` compiles and runs
- [ ] `examples/c/custom_draw.c` compiles and runs
- [ ] Header verified with MSVC, Clang, MinGW
- [ ] Pre-push script runs successfully and tests the C examples

## Open Questions

### 1. Custom draw API surface

The escape hatch gives raw rendering access. How much to expose?

- **A. Thin wrapper: rect, text, line, ellipse, image (Recommended)** - Cover the common cases with safe C functions. Don't expose the full Direct2D API. Keeps the C header manageable (under 20 drawing functions) and avoids leaking D2D types into the public API. Sufficient for custom HUDs, charts, and visualizations.
- **B. Pass-through D2D render target pointer** - `winpane_surface_get_d2d_target()` returns an `ID2D1DeviceContext*`. Maximum power, zero abstraction. But ties the public API to Direct2D, makes the header Windows-specific (consumers need D2D headers), and breaks if the rendering backend ever changes.
- **C. Both** - Thin wrapper as default, raw D2D pointer as opt-in behind a separate function. More API surface but satisfies both audiences.

> **Decision: A.** 10 drawing functions (clear, fill/stroke rect, text, line, fill/stroke ellipse, image, fill/stroke rounded rect). Keeps the header portable. Raw D2D access can be added later behind a separate opt-in function if demand exists.

### 2. Struct versioning strategy

Config structs cross the FFI boundary. How to handle forward compatibility when new config fields are added in future versions?

- **A. Version field + size field (Recommended)** - Every config struct starts with `uint32_t version; uint32_t size;`. SDK checks version and only reads fields it knows about. New fields are always added at the end of the struct. Old binaries with smaller structs still work. This is the standard COM-era pattern used by DirectX, Win32 `OPENFILENAME`, etc.
- **B. Builder functions** - No structs cross the boundary. `winpane_config_create() -> config*`, `winpane_config_set_width(config, 800)`, etc. Fully extensible without versioning concerns. More verbose but impossible to break.
- **C. JSON config strings** - `winpane_create("{\"width\":800}")`. Maximum flexibility but terrible ergonomics for C consumers and adds a JSON parser dependency.

> **Decision: A.** Version + size on config structs (HudConfig, PanelConfig, TrayConfig). Value-only structs (Color, elements, MenuItem, Event) are frozen per major version and don't use versioning.

## Notes

- The FFI crate is the stable public interface for non-Rust consumers. The Rust API (in `winpane` crate) can evolve faster. The C ABI should be conservative with breaking changes.
- cbindgen config should exclude internal types. Only `#[repr(C)]` types with appropriate annotations get exported to the header.
- Consider shipping a `winpane.def` file alongside `.dll` for consumers who need it.
- The custom draw escape hatch only works in-process (same address space). It will not be available over the IPC/stdio protocol added in P5. Document this clearly.
