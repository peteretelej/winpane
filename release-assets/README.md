# winpane SDK

Windows overlay SDK for creating HUDs, panels, widgets, and tray indicators.

## Contents

- `winpane-host.exe` - JSON-RPC host process for language-agnostic integration
- `winpane.dll` - Native library for C/C++ or FFI consumers
- `winpane.lib` - Import library for linking
- `winpane.h` - C header file

## Quick Start

Verify the installation:

```powershell
.\winpane-host.exe --version
```

## Documentation

Full documentation: https://github.com/peteretelej/winpane

- Rust SDK: `docs/guides/rust.md`
- Node.js SDK: `docs/guides/nodejs.md`
- C SDK: `docs/guides/c.md`
- JSON-RPC protocol: `docs/protocol.md`

## Note

This binary is not code-signed. Windows SmartScreen may show a warning on first run. Click "More info" then "Run anyway" to proceed.
