<!-- AGENTS.md must remain concise - critical info & compacted indexes only, no verbose explanations -->

# winpane

Windows overlay SDK. Out-of-process DirectComposition surfaces (HUD, Panel, PiP, Tray) with retained-mode scene graph. Rust core, C ABI, Node.js, JSON-RPC CLI host.

## Docs

| Path | Contents |
|------|----------|
| `docs/design.md` | High-level design, key decisions |
| `docs/design/threading.md` | Engine thread, command queue, event polling |
| `docs/design/rendering.md` | D3D11/DXGI/D2D/DirectComposition pipeline |
| `docs/design/surfaces.md` | Surface types, lifecycle, config |
| `docs/design/input.md` | Hit testing, interactive elements, drag |
| `docs/design/ffi.md` | C ABI conventions, error handling, type mapping |
| `docs/guides/rust.md` | Rust SDK walkthrough |
| `docs/guides/nodejs.md` | Node.js SDK walkthrough |
| `docs/guides/typescript.md` | TypeScript/JavaScript walkthrough |
| `docs/guides/c.md` | C SDK walkthrough |
| `docs/guides/go.md` | Go walkthrough (cgo or syscall.LoadDLL) |
| `docs/guides/zig.md` | Zig walkthrough (@cImport or DynLib) |
| `docs/guides/python.md` | Python/CLI host walkthrough |
| `docs/cookbook.md` | 10 self-contained recipes (Rust + Node.js + JSON-RPC) |
| `docs/protocol.md` | JSON-RPC 2.0 reference for winpane-host |
| `docs/limitations.md` | 9 known constraints with workarounds |
| `docs/signing.md` | Code signing, SmartScreen, MSIX |

## Code entry points

| Path | Role |
|------|------|
| `crates/winpane/src/lib.rs` | Public Rust API (Context, Hud, Panel, Pip, Tray) |
| `crates/winpane-core/src/engine.rs` | Engine thread, command dispatch, surface management |
| `crates/winpane-core/src/renderer.rs` | D2D/D3D11 rendering, scene graph draw |
| `crates/winpane-core/src/window.rs` | HWND creation, window procedures |
| `crates/winpane-core/src/scene.rs` | Scene graph (IndexMap of elements) |
| `crates/winpane-core/src/input.rs` | HitTestMap, PanelState, hover tracking |
| `crates/winpane-core/src/tray.rs` | System tray icon, Shell_NotifyIconW |
| `crates/winpane-core/src/types.rs` | All public types (elements, configs, events, enums) |
| `crates/winpane-core/src/monitor.rs` | DPI handling, multi-monitor support |
| `crates/winpane-core/src/command.rs` | Command enum for engine IPC |
| `crates/winpane-ffi/src/lib.rs` | C ABI (35 extern "C" functions, cbindgen) |
| `crates/winpane-host/src/dispatch.rs` | JSON-RPC method dispatcher |
| `bindings/node/src/lib.rs` | napi-rs Node.js addon (WinPane class) |

## Build

```sh
cargo build --workspace --all-targets   # full build (Windows only)
cargo fmt --all -- --check              # format check (works on macOS)
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Architecture summary

- Engine thread owns all HWNDs and GPU resources
- Consumer communicates via MPSC command channel; PostMessageW wakes the loop
- Events polled via separate MPSC receiver (no callbacks)
- Rendering: D3D11 -> DXGI swap chain -> D2D device context -> DirectComposition visual
- Scene graph: IndexMap<String, Element> per surface, dirty flag avoids unnecessary draws
- WS_EX_NOREDIRECTIONBITMAP + DXGI_ALPHA_MODE_PREMULTIPLIED for transparency
- Min OS: Windows 10 1903+; backdrop effects require Win11 22H2+
- Large files or precision edits: use largefile MCP server if available (`get_overview`, `search_content`, `read_content`, `edit_content`) - largefile requires absolute paths
