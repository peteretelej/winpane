# P3: C ABI & FFI - Breakdown Review

Review this plan breakdown for a software project. You have the proposal (source of truth) and all phase specs/plans below.

Check for:
1. **Requirements coverage**: Are all proposal requirements covered? Any unjustified extra work? Is phase ordering sound?
2. **Correctness**: Does each phase's approach actually solve what its spec describes? Wrong assumptions? Logical errors?
3. **Clarity**: Could a developer pick up each phase and start without guessing? Ambiguous terms or hand-wavy steps?
4. **Simplicity**: Over-engineering, unnecessary abstraction, premature optimization?
5. **Source code accuracy**: Read the referenced source files and verify that file paths, function names, types, CSS classes, store fields, hook signatures, etc. mentioned in the specs actually exist and behave as described.
6. **Outdated patterns**: If specs reference libraries, APIs, or patterns that seem suspect, do a web search to verify they're current.

For each issue found, be concrete: state the file, what's wrong, and what it should say instead. "Needs more detail" is not useful. "phase-2/spec.md should specify the REST endpoint path and method" is.

Organize your output as:
- **Requirements Coverage**: gaps, extras, ordering issues
- **Phase-by-phase findings**: issues per phase with specific fix suggestions
- **Cross-cutting concerns**: issues that span multiple phases

Write your findings to `agi-{agent}.md` in the working directory (replace {agent} with your name: claude or kimi).

---

## PROPOSAL.MD (source of truth)

```markdown
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

Four opaque handle types:
- `winpane_context_t*` - wraps `winpane::Context`
- `winpane_surface_t*` - wraps either `winpane::Hud` or `winpane::Panel` (unified)
- `winpane_tray_t*` - wraps `winpane::Tray`
- `winpane_canvas_t*` - temporary draw accumulator, valid between begin/end_draw

### Unified surface handle

Hud and Panel have identical element/surface operations. The FFI layer uses a single `winpane_surface_t*` handle backed by an internal enum `FfiSurface { Hud(winpane::Hud), Panel(winpane::Panel) }`. Creation is type-specific, everything after uses the unified handle.

### Error handling

Every `extern "C"` function returns `int32_t`:
- `0` = success
- `-1` = error (details via `winpane_last_error()`)
- `-2` = panic caught at FFI boundary

### Struct versioning

Config structs use version + size fields. Value-only structs (Color, elements, MenuItem, Event) do NOT use versioning.

### Custom draw pipeline

Buffered command list pattern: `begin_draw` creates an accumulator, canvas calls push DrawOps, `end_draw` sends ops to engine thread. Engine executes on D2D render target.

### cbindgen header generation

`cbindgen` runs via `build.rs` in `winpane-ffi` crate to generate `include/winpane.h`.

## Public API

### Types

- Opaque handles: winpane_context_t, winpane_surface_t, winpane_tray_t, winpane_canvas_t
- Color: winpane_color_t { r, g, b, a: uint8_t }
- Config structs (versioned): winpane_hud_config_t, winpane_panel_config_t, winpane_tray_config_t
- Element structs (frozen): winpane_text_element_t, winpane_rect_element_t, winpane_image_element_t
- Menu: winpane_menu_item_t
- Events: winpane_event_type_t enum, winpane_mouse_button_t enum, winpane_event_t struct

### Functions (35 total)

1 error, 2 context, 2 surface creation, 11 surface ops, 6 tray, 1 event, 2 draw lifecycle, 10 canvas drawing.

Key detail for poll_event:
> For `poll_event`: returns 0 and fills the event struct if an event is available, returns 1 if no event (WINPANE_EVENT_NONE).

### Verification

P3 is complete when:
1. `cargo build --workspace` passes on CI, producing `winpane.dll` + `winpane.lib`
2. `cargo clippy --workspace -- -D warnings` passes
3. `cargo test --workspace` passes
4. `include/winpane.h` compiles without warnings on MSVC, Clang, MinGW
5. C examples compile, link, and run
6. Rust custom_draw example runs
7. `winpane_last_error()` returns meaningful errors
8. All 35 functions present in DLL exports
9. No panics escape FFI boundary
10. `cargo fmt --all -- --check` passes
```

---

## PLAN.MD (phase overview)

```markdown
- [ ] Phase 1: [Custom Draw Core](1-custom-draw-core/spec.md) - DrawOp type, Command variant, renderer execution, public Rust API, Rust example
- [ ] Phase 2: [FFI Crate Setup](2-ffi-crate-setup/spec.md) - Cargo.toml, cbindgen, build.rs, error handling infra, helper utilities
- [ ] Phase 3: [FFI Types and Handles](3-ffi-types-and-handles/spec.md) - repr(C) types, conversions, opaque handles, context lifecycle, event polling
- [ ] Phase 4: [FFI Functions](4-ffi-functions/spec.md) - Surface creation/ops, tray functions, canvas functions
- [ ] Phase 5: [Examples and CI](5-examples-and-ci/spec.md) - C examples, CMake/build.bat, CI header check, .def file, progress update
```

---

## PHASE 1 SPEC: Custom Draw Core and Rust API

Key items:
1. DrawOp enum in types.rs (10 variants: Clear, FillRect, StrokeRect, DrawText, DrawLine, FillEllipse, StrokeEllipse, DrawImage, FillRoundedRect, StrokeRoundedRect)
2. Command::CustomDraw variant in command.rs
3. execute_draw_ops + execute_single_draw_op methods on SurfaceRenderer in renderer.rs
4. CustomDraw handler in engine.rs
5. Re-export DrawOp from winpane-core lib.rs (automatic via pub use types::*)
6. custom_draw() method on Hud and Panel in winpane/src/lib.rs
7. Rust example: examples/rust/custom_draw.rs

Referenced source files:
- `crates/winpane-core/src/types.rs` - DrawOp goes after ImageElement (line 118)
- `crates/winpane-core/src/command.rs` - DrawOp import line 4, new variant after DestroyTray (line 67)
- `crates/winpane-core/src/renderer.rs` - import line 11, new methods after set_opacity (starts line 475)
- `crates/winpane-core/src/engine.rs` - new arm in command match after DestroyTray (line 300)
- `crates/winpane/src/lib.rs` - re-export line 5-8, custom_draw after Hud.id() (line 169), Panel.id() (line 258)

D2D_POINT_2F note: Check existing code for correct point type import in windows-rs 0.62.

---

## PHASE 2 SPEC: FFI Crate Setup and Error Handling

Key items:
1. Update Cargo.toml: cdylib + staticlib, winpane dep, cbindgen build-dep
2. Create cbindgen.toml: C language, type renames, enum style
3. Create build.rs: header generation to include/winpane.h
4. Error handling in lib.rs: thread-local storage, set_last_error, winpane_last_error(), ffi_try!/ffi_try_with! macros, null helpers, cstr_to_string

Referenced source files:
- `crates/winpane-ffi/Cargo.toml` (current stub - 10 lines)
- `crates/winpane-ffi/src/lib.rs` (current stub - 2 lines)

---

## PHASE 3 SPEC: FFI Types and Opaque Handles

Key items:
1. WINPANE_CONFIG_VERSION constant
2. WinpaneColor repr(C) with to_rust()
3. Versioned config structs: WinpaneHudConfig, WinpanePanelConfig, WinpaneTrayConfig
4. Element structs: WinpaneTextElement, WinpaneRectElement, WinpaneImageElement
5. WinpaneMenuItem, event types/enums
6. Opaque handles: FfiSurface enum, WinpaneContext, WinpaneSurface, WinpaneTray, CanvasAccumulator, WinpaneCanvas
7. winpane_create, winpane_destroy
8. winpane_poll_event

IMPORTANT - poll_event semantics:
- Spec says: "If Some(e): returns 0; If None: returns 0"
- But proposal says: "returns 0 if event available, returns 1 if no event"
- The C example in proposal: `if (winpane_poll_event(ctx, &event) == 0) { handle event }`
- This is an inconsistency to check.

Referenced Rust types (verify these match):
- winpane::Color::rgba(r, g, b, a)
- winpane::HudConfig { x, y, width, height }
- winpane::PanelConfig { x, y, width, height, draggable, drag_height }
- winpane::TrayConfig { icon_rgba, icon_width, icon_height, tooltip }
- winpane::TextElement { text, x, y, font_size, color, font_family, bold, italic, interactive }
- winpane::RectElement { x, y, width, height, fill, corner_radius, border_color, border_width, interactive }
- winpane::ImageElement { x, y, width, height, data, data_width, data_height, interactive }
- winpane::Event { ElementClicked{surface_id, key}, ElementHovered{...}, ElementLeft{...}, TrayClicked{button}, TrayMenuItemClicked{id} }
- winpane::SurfaceId(pub u64)
- winpane::MouseButton { Left, Right, Middle }
- winpane::MenuItem { id, label, enabled }

---

## PHASE 4 SPEC: FFI Functions

Key items:
1. Surface creation: winpane_hud_create, winpane_panel_create
2. Surface operations (11): destroy, id, set_text, set_rect, set_image, remove, show, hide, set_position, set_size, set_opacity
3. Tray (6): create, destroy, set_tooltip, set_icon, set_popup, set_menu
4. Canvas (12): begin_draw, end_draw, clear, fill_rect, stroke_rect, draw_text, draw_line, fill_ellipse, stroke_ellipse, draw_image, fill_rounded_rect, stroke_rounded_rect

Key concern: tray_set_popup extracts Panel from FfiSurface enum, errors if Hud.

---

## PHASE 5 SPEC: C Examples, CI, and Verification

Key items:
1. hello_hud.c - retained-mode API demo
2. custom_draw.c - canvas API demo
3. CMakeLists.txt
4. build.bat
5. Delete .gitkeep
6. winpane.def (35 symbols)
7. CI header verification step
8. Update phases-progress.md

CI note: Current ci.yml does NOT use ilammy/msvc-dev-cmd. The spec says to add `cl /c /W4 ...` which needs MSVC on PATH.

---

## REFERENCED SOURCE FILES (current state)

### crates/winpane-core/src/types.rs
- Color at line 9 with rgba(), rgb(), to_d2d_premultiplied()
- TextElement at line 61: { text, x, y, font_size, color, font_family: Option<String>, bold: bool, italic: bool, interactive: bool }
- RectElement at line 92: { x, y, width, height, fill, corner_radius, border_color: Option<Color>, border_width, interactive: bool }
- ImageElement at line 107: { x, y, width, height, data: Vec<u8>, data_width: u32, data_height: u32, interactive: bool }
- HudConfig at line 122: { x: i32, y: i32, width: u32, height: u32 }
- PanelConfig at line 133: { x: i32, y: i32, width: u32, height: u32, draggable: bool, drag_height: u32 }
- TrayConfig at line 146: { icon_rgba: Vec<u8>, icon_width: u32, icon_height: u32, tooltip: String }
- Event at line 163: ElementClicked{surface_id: SurfaceId, key: String}, ElementHovered{...}, ElementLeft{...}, TrayClicked{button: MouseButton}, TrayMenuItemClicked{id: u32}
- MouseButton at line 174: { Left, Right, Middle }
- MenuItem at line 183: { id: u32, label: String, enabled: bool }
- Error at line 192: { WindowCreation, DeviceCreation, SwapChainCreation, RenderError, ThreadSpawnFailed, SurfaceNotFound, Shutdown }
- SurfaceId at line 221: SurfaceId(pub u64)
- TrayId at line 158: TrayId(pub u64)

### crates/winpane-core/src/command.rs
- Line 4: `use crate::types::{Error, HudConfig, MenuItem, PanelConfig, SurfaceId, TrayConfig, TrayId};`
- Command enum: CreateHud, SetElement, RemoveElement, Show, Hide, SetPosition, SetSize, SetOpacity, DestroySurface, Shutdown, CreatePanel, CreateTray, SetTrayTooltip, SetTrayIcon, SetTrayPopup, SetTrayMenu, DestroyTray
- DestroyTray at line 67

### crates/winpane-core/src/engine.rs (561 lines)
- DestroyTray handler at line 300
- Command match block approximately lines 166-305

### crates/winpane-core/src/renderer.rs (526 lines)
- Import line 11: `use crate::types::{Error, ImageElement, RectElement, TextElement};`
- render_text at line 314
- set_opacity at line 475

### crates/winpane/src/lib.rs
- Re-export line 5-8: Color, Error, Event, HudConfig, ImageElement, MenuItem, MouseButton, PanelConfig, RectElement, SurfaceId, TextElement, TrayConfig, TrayId
- Context::new() at line 24
- Context::create_hud() at line 31
- Context::create_panel() at line 45
- Context::create_tray() at line 62
- Context::poll_event() at line 79 - returns Option<Event>
- Hud impl: set_text, set_rect, set_image, remove, show, hide, set_position, set_size, set_opacity, id()
- Hud.id() at line 167-169
- Panel impl: identical methods
- Panel.id() at line 256-258
- Tray: set_tooltip(&str), set_icon(Vec<u8>, u32, u32), set_popup(&Panel), set_menu(Vec<MenuItem>)

### crates/winpane-ffi/Cargo.toml (current stub)
10-line stub with name, version, edition, license, repository, description, lints

### crates/winpane-ffi/src/lib.rs (current stub)
2-line comment: `// winpane-ffi: C ABI bindings (cdylib). Stub for now.`

### crates/winpane/Cargo.toml
Dependencies: winpane-core
Examples: hud_demo, interactive_panel, tray_ticker (paths: ../../examples/rust/*.rs)

### .github/workflows/ci.yml
windows-latest, stable toolchain with clippy+rustfmt, format check, clippy, build, test
NO ilammy/msvc-dev-cmd action currently

### examples/c/.gitkeep exists
