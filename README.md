# winpane

A Rust SDK for creating companion UI surfaces on Windows - overlays, HUDs, panels, widgets, PiP thumbnails, tray indicators.

Uses out-of-process DirectComposition windows (no process injection) with a retained-mode API consumable from Rust, C/C++, Node.js, or any language via C ABI.

## Status

Early development. Not yet usable.

## Architecture

- **Rendering**: `WS_EX_NOREDIRECTIONBITMAP` + DirectComposition + Direct2D + DirectWrite. GPU-native, no legacy `WS_EX_LAYERED` / `UpdateLayeredWindow` path.
- **API**: Retained-mode scene graph with element keys. SDK thread owns all HWNDs and the Win32 message loop.
- **FFI**: C ABI with opaque handles (cbindgen headers), napi-rs for Node.js/Bun, CLI/stdio JSON-RPC as universal fallback.
- **Target OS**: Windows 10 1903+

## Crates

| Crate | Description |
|-------|-------------|
| `winpane-core` | Internal Win32/DirectComposition logic |
| `winpane` | Public Rust API |
| `winpane-ffi` | C ABI bindings (cdylib + cbindgen) |
| `winpane-host` | CLI/stdio JSON-RPC binary |

## License

MIT
