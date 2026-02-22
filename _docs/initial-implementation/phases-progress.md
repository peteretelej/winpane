# winpane: Phase Progress

Shared state file for sequential phase execution. **Every phase must read this file before starting and update it after completing.**

## Project Summary

**winpane** is an MIT-licensed Rust SDK for creating companion UI surfaces on Windows (overlays, HUDs, panels, widgets, PiP thumbnails, tray indicators). It uses out-of-process DirectComposition windows (no process injection) with a retained-mode API consumable from any language via C ABI.

- **Repo**: Cargo workspace monorepo
- **Crates**: `winpane-core` (internal), `winpane` (public Rust API), `winpane-ffi` (C ABI cdylib), `winpane-host` (CLI binary)
- **npm**: `winpane` (napi-rs addon)
- **Rendering**: `WS_EX_NOREDIRECTIONBITMAP` + DirectComposition + Direct2D + DirectWrite
- **Target audience**: Developer tool authors (AI assistants, dev tools, status bars)
- **Min OS**: Windows 10 1903+ (practical target)
- **License**: MIT

## Phase Order

| Phase | Folder | Status | Summary |
|-------|--------|--------|---------|
| P0 | `p0-bootstrap` | **Complete** | Workspace, CI, hello-world transparent window |
| P1 | `p1-hud` | **Complete** | HUD overlay + rendering pipeline + Rust API |
| P2 | `p2-interactive` | **Complete** | Interactive panels + system tray ticker |
| P3 | `p3-ffi` | **Complete** | C ABI + cbindgen + custom draw escape hatch |
| P4 | `p4-advanced-surfaces` | Not started | PiP thumbnail, anchored companion, capture exclusion |
| P5 | `p5-host` | Not started | CLI/stdio host + npm package via napi-rs |
| P6 | `p6-polish` | Not started | Backdrop effects, device loss recovery, DPI hardening, docs |

## Pre-push Checks (All Phases)

**Every phase must run these checks locally before pushing.** This is a mandatory final step.

```bash
# Format check - the only CI check that works on macOS
cargo fmt --all -- --check

# Fix if needed
cargo fmt --all

# Verify staged files look correct
git diff --cached --name-only
```

`cargo clippy`, `cargo build`, and `cargo test` require Windows and only run in CI. Format issues are the cheapest to catch locally and the most wasteful to discover in CI. Run `cargo fmt` before every push.

## Completed Phase Notes

### P0: Bootstrap (Complete)

**What was built:**
- Cargo workspace with 4 crates: `winpane-core`, `winpane`, `winpane-ffi` (stub), `winpane-host` (stub)
- `hello_transparent` example proving the full DirectComposition rendering pipeline
- GitHub Actions CI on `windows-latest` (fmt, clippy, build, test)
- MIT license, .gitignore, README, npm placeholder

**Key files:**
- `Cargo.toml` - workspace root, `windows` 0.62 via `[workspace.dependencies]`
- `crates/winpane-core/Cargo.toml` - windows-rs features for D3D11, DXGI, DirectComposition, Direct2D
- `examples/rust/hello_transparent.rs` - end-to-end pipeline proof-of-concept
- `.github/workflows/ci.yml` - CI pipeline

**Names reserved:**
- crates.io: `winpane-core` 0.0.1, `winpane` 0.0.1, `winpane-ffi` 0.0.1
- npm: `winpane` 0.0.1

**Verified:**
- Blue circle renders on Win11 with per-pixel transparency, click-through, topmost, no taskbar entry
- CI passes on `windows-latest` (fmt, clippy, build, test)

**Gotchas for P1:**
- `windows-rs` 0.62 replaces `D2D_POINT_2F` with `Vector2` from `windows-numerics` crate. Use `windows_numerics::Vector2` for all D2D point types.
- `cargo publish` fails on macOS due to `windows-future`/`windows-core` version conflict. Use `--no-verify` since CI validates the Windows build.
- `ShowWindow` returns a `BOOL` that must be consumed (`let _ = ...`) or clippy errors with `-D warnings`.
- D3D11 device creation should fall back to `D3D_DRIVER_TYPE_WARP` for CI runners and VMs without a GPU.

### P2: Interactive Surfaces (Complete)

**What was built:**
- Interactive `Panel` surface with selective click-through via `WM_NCHITTEST` hit-testing
- `HitTestMap` for mapping mouse coordinates to interactive elements
- System `Tray` icon with HICON creation from RGBA, popup panel, and native context menu
- Event channel (engine -> consumer) with `poll_event()` polling API
- `PanelState` per-window state with hover tracking and drag support

**Key files:**
- `crates/winpane-core/src/input.rs` - HitTestMap, PanelState, element bounds
- `crates/winpane-core/src/tray.rs` - HICON creation, Shell_NotifyIconW, TrackPopupMenu
- `crates/winpane-core/src/engine.rs` - SurfaceKind, event channel, panel/tray creation
- `crates/winpane-core/src/window.rs` - panel_wndproc, WM_TRAY_CALLBACK handling
- `crates/winpane-core/src/types.rs` - PanelConfig, TrayConfig, Event, MouseButton, MenuItem
- `crates/winpane/src/lib.rs` - Panel, Tray structs, Context::create_panel/create_tray/poll_event

**API surface:**
- `Context::create_panel(PanelConfig)` - creates interactive panel
- `Context::create_tray(TrayConfig)` - creates system tray icon
- `Context::poll_event()` - polls for mouse/tray events
- `Panel` - same element API as Hud, plus `id()` for event matching
- `Tray` - `set_tooltip`, `set_icon`, `set_popup(&Panel)`, `set_menu(Vec<MenuItem>)`
- `Event` enum: `ElementClicked`, `ElementHovered`, `ElementLeft`, `TrayClicked`, `TrayMenuItemClicked`

**Gotchas for P3:**
- `Event` enum uses `String` keys and heap-allocated variants; will need C-compatible representation for FFI (likely separate C event struct with fixed-size buffers or callback approach)
- `interactive: bool` on elements is ignored for HUD surfaces, only meaningful on Panel surfaces
- `TrayId` is separate from `SurfaceId`; FFI layer needs to handle both ID types
- `Panel` drop sends `DestroySurface` (same as Hud), but `Tray` drop sends `DestroyTray`

### P3: C ABI & FFI (Complete)

**What was built:** winpane-ffi cdylib with 35 extern "C" functions, auto-generated winpane.h via cbindgen, custom draw escape hatch (DrawOp pipeline), thread-local error handling, versioned config structs, unified surface handle.

**Key files:**
- `crates/winpane-ffi/src/lib.rs` - all 35 FFI functions, repr(C) types, opaque handles
- `crates/winpane-ffi/cbindgen.toml` - header generation config
- `crates/winpane-ffi/build.rs` - cbindgen integration
- `crates/winpane-ffi/include/winpane.h` - auto-generated C header
- `crates/winpane-ffi/include/winpane.def` - DLL export definitions
- `crates/winpane-core/src/types.rs` - DrawOp enum
- `crates/winpane-core/src/renderer.rs` - execute_draw_ops, execute_single_draw_op
- `examples/c/hello_hud.c`, `examples/c/custom_draw.c` - C examples
- `examples/rust/custom_draw.rs` - Rust custom draw example

**API surface:** 35 functions (1 error, 2 context, 2 surface creation, 11 surface ops, 6 tray, 1 event, 2 draw lifecycle, 10 canvas drawing).

**Gotchas for P4:**
- Custom draw is in-process only, not available over IPC.
- The canvas handle is invalid after end_draw.
- DrawOp is fire-and-forget (scene graph changes overwrite custom draw content).
- Config struct versioning is forward-compatible but element structs are frozen.

## Key Technical Decisions

These decisions are final. Do not revisit.

1. **DirectComposition only** - No legacy `WS_EX_LAYERED` / `UpdateLayeredWindow` path. `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` fails on layered windows on Win11.
2. **Out-of-process only** - No DLL injection, no render hook, no in-process overlay. All surfaces are topmost Win32 windows.
3. **Retained-mode API** - Scene graph (text, shapes, images) with element keys. Not immediate-mode. SDK manages dirty-region tracking internally.
4. **Internal SDK thread** - SDK spawns its own thread with Win32 message loop. Consumer thread communicates via lock-free command queue. Consumer never pumps messages.
5. **Unsigned distribution** - No EV code signing for now. Docs guide developers on signing their own distributions.
