---
name: repo-guide
description: Evergreen guide to the winpane codebase - architecture, build and verify loop, conventions, and where to make changes
---

# Repo Guide

Reference for working in the winpane repo: architecture, verify loop,
conventions, and where to make changes. Load this any time you work here,
whether you are new to the codebase or returning to an unfamiliar area.

## 1. What winpane is

Windows overlay SDK. Creates transparent, always-on-top surfaces (Hud, Panel,
Pip, Tray) as out-of-process DirectComposition windows, driven by a
retained-mode scene graph. One Rust core, four frontends: typed Rust API, C ABI
(cdylib + cbindgen header), napi-rs Node addon, JSON-RPC CLI host (any
language). Windows 10 1903+ only, by design.

## 2. Read in this order

1. `AGENTS.md` - repo map: docs index, code entry points, build commands
2. `docs/design.md` - architecture and the key decisions
3. Design details as needed: `docs/design/threading.md` (engine thread,
   command/event channels), `rendering.md` (D3D11/D2D/DComp pipeline),
   `surfaces.md`, `input.md`, `ffi.md`, `style.md`
4. `docs/guides/<lang>.md` for the frontend you are touching
5. `docs/limitations.md` - known constraints and workarounds

## 3. Mental model

- `Context::new()` spawns one engine thread that owns every HWND and all GPU
  resources. Consumer threads never touch Win32 or D3D directly.
- Commands flow in via `mpsc::Sender<Command>` plus a `PostMessageW` wake on a
  message-only control window. Surface creation blocks on a reply channel.
- Events flow out via a second channel; consumers poll `poll_event()`. No
  callbacks anywhere.
- Window procs write to thread-local queues (DPI changes, tray events, fade
  completions) drained after each `DispatchMessageW`.
- Scene graph is an `IndexMap<String, Element>` per surface; a dirty flag
  skips redraws when nothing changed.

## 4. Where things live

| Path | Role |
|------|------|
| `crates/winpane/src/lib.rs` | Public Rust API (Context, Hud, Panel, Pip, Tray) |
| `crates/winpane-core/src/engine.rs` | Engine thread, command dispatch, surface table |
| `crates/winpane-core/src/renderer.rs` | D2D/D3D11 draw, scene graph rendering |
| `crates/winpane-core/src/window.rs` | HWND creation, window procedures |
| `crates/winpane-core/src/scene.rs` | Scene graph (IndexMap of elements) |
| `crates/winpane-core/src/types.rs` | Public types: elements, configs, events, Error |
| `crates/winpane-ffi/src/lib.rs` | C ABI (extern "C" fns, cbindgen) |
| `crates/winpane-host/src/dispatch.rs` | JSON-RPC method dispatcher |
| `bindings/node/src/lib.rs` | napi-rs addon (WinPane class) |

## 5. Build and verify

```sh
cargo build --workspace --all-targets   # Windows only
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

MSRV is 1.88 (CI pins a job to it). `scripts/pre-push` runs a fast gate (fmt,
clippy, `cargo check`); tests run in CI. Symlink it as a hook (see
CONTRIBUTING.md). Try the SDK end to end with
`cargo run -p winpane --example clock_overlay`.

## 6. Conventions

- All Win32/D3D/D2D calls stay in winpane-core; the public crate is pure Rust.
- Workspace lints live in root `Cargo.toml`; CI runs clippy with
  `-D warnings`: no `dbg!`, no stdout/stderr prints, every unsafe block needs
  a `// SAFETY:` comment.
- One `Error` enum (winpane-core types.rs) is surfaced through all frontends.
- Versions bump in lockstep across 8 files; use the `bump-version` skill.
- `Cargo.lock` is gitignored; committed files never depend on gitignored
  scratch such as `_docs/`.
- The Node addon (`bindings/node`) is its own cargo workspace (excluded from
  the root); build with `npm run build` inside that directory.

## 7. Making a change (touch paths)

- **New surface operation**: `types.rs` -> `command.rs` -> `engine.rs`
  dispatch -> (`renderer.rs` if it draws) -> public API in
  `crates/winpane/src/lib.rs` -> frontends: ffi (`winpane-ffi` + regen
  header), host (`dispatch.rs` + `docs/protocol.md`), node
  (`bindings/node/src/lib.rs` + `index.d.ts`).
- **New scene element**: `types.rs` (element struct) -> `scene.rs` ->
  `renderer.rs` draw arm -> public API setters -> frontends.
- **New example**: `examples/rust/<name>.rs` plus an `[[example]]` entry in
  `crates/winpane/Cargo.toml`.
- **Docs**: `docs/` rebuilds the site automatically on push
  (starlight-action); update the relevant guide or `docs/protocol.md` in the
  same change as the code.
