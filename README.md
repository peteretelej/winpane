# winpane

Windows overlay SDK for companion UI surfaces. Create floating HUDs, interactive panels, PiP thumbnails, and system tray icons using DirectComposition, Direct2D, and a retained-mode scene graph. No process injection, no legacy `UpdateLayeredWindow` - GPU-native per-pixel transparency via `WS_EX_NOREDIRECTIONBITMAP`.

## Features

- **Surface types** - HUD (click-through), Panel (interactive with hit testing), PiP (live DWM thumbnails), Tray (system tray icons with popups and menus)
- **Scene graph** - Retained-mode text, rect, and image elements with string keys and insertion-order z-ordering
- **Input** - Selective click-through, interactive element click/hover events, drag handles
- **Anchoring** - Attach surfaces to window corners with automatic position tracking
- **Capture exclusion** - Hide surfaces from screenshots and screen sharing (Win10 2004+)
- **Backdrop effects** - DWM Mica and Acrylic (Win11 22H2+)
- **Fade animations** - DirectComposition opacity transitions
- **Custom draw** - Escape hatch for procedural Direct2D rendering (in-process only)
- **GPU recovery** - Automatic device loss detection and surface recovery

## Quickstart: Rust

```rust
use winpane::{Color, Context, HudConfig, RectElement, TextElement};

fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;
    let hud = ctx.create_hud(HudConfig {
        x: 100, y: 100, width: 300, height: 100,
    })?;

    hud.set_rect("bg", RectElement {
        x: 0.0, y: 0.0, width: 300.0, height: 100.0,
        fill: Color::rgba(20, 20, 30, 200), corner_radius: 8.0,
        ..Default::default()
    });
    hud.set_text("msg", TextElement {
        text: "Hello from winpane".into(),
        x: 16.0, y: 16.0, font_size: 18.0,
        color: Color::WHITE,
        ..Default::default()
    });
    hud.show();

    loop { std::thread::sleep(std::time::Duration::from_secs(1)); }
}
```

## Quickstart: Node.js

```js
const { WinPane } = require("winpane");

const wp = new WinPane();
const hud = wp.createHud({ width: 300, height: 100, x: 100, y: 100 });

wp.setRect(hud, "bg", {
  x: 0, y: 0, width: 300, height: 100,
  fill: "#14141ec8", cornerRadius: 8,
});
wp.setText(hud, "msg", {
  text: "Hello from winpane",
  x: 16, y: 16, fontSize: 18,
});
wp.show(hud);
```

## Quickstart: C

```c
#include "winpane.h"

int main(void) {
    WinpaneContext *ctx;
    if (winpane_create(&ctx) != 0) return 1;

    WinpaneHudConfig cfg = { .size = sizeof(cfg), .x = 100, .y = 100, .width = 300, .height = 100 };
    WinpaneSurface *hud;
    winpane_hud_create(ctx, &cfg, &hud);

    WinpaneTextElement text = {
        .text = "Hello from winpane", .x = 16, .y = 16,
        .font_size = 18, .color = { 255, 255, 255, 255 },
    };
    winpane_surface_set_text(hud, "msg", &text);
    winpane_surface_show(hud);

    Sleep(INFINITE);
    winpane_surface_destroy(hud);
    winpane_destroy(ctx);
    return 0;
}
```

## Quickstart: Python via CLI host

```python
import subprocess, json

proc = subprocess.Popen(
    ["winpane-host"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE,
    text=True, bufsize=1,
)

def rpc(method, params, id):
    msg = json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": id})
    proc.stdin.write(msg + "\n")
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())

result = rpc("create_hud", {"x": 100, "y": 100, "width": 300, "height": 100}, 1)
sid = result["result"]["surface_id"]

rpc("set_text", {"surface_id": sid, "key": "msg", "text": "Hello from winpane", "x": 16, "y": 16, "font_size": 18}, 2)
rpc("show", {"surface_id": sid}, 3)
```

## Surface types

| Type | Description |
|------|-------------|
| **Hud** | Click-through overlay for passive information display |
| **Panel** | Interactive surface with selective hit testing, click/hover events, and drag support |
| **Pip** | Live DWM thumbnail of another window (Picture-in-Picture) |
| **Tray** | System tray icon with popup panel association and right-click context menu |

## Documentation

- [Architecture](docs/architecture.md) - Crate structure, threading model, rendering pipeline
- [Cookbook](docs/cookbook.md) - 10 self-contained recipes for common patterns
- [Protocol Reference](docs/protocol.md) - JSON-RPC 2.0 protocol for the CLI host
- [Signing & Distribution](docs/signing.md) - Code signing, SmartScreen, MSIX packaging
- [Limitations](docs/limitations.md) - Known constraints and workarounds

## Crates

| Crate | Description |
|-------|-------------|
| `winpane-core` | Internal Win32/DirectComposition implementation |
| `winpane` | Public Rust API |
| `winpane-ffi` | C ABI bindings (cdylib + cbindgen) |
| `winpane-host` | CLI/stdio JSON-RPC host binary |
| `bindings/node` | Node.js/Bun native addon (napi-rs) |

## Platform requirements

- Windows 10 version 1903 or later
- Windows 11 version 22H2 or later for backdrop effects (Mica, Acrylic)
- Windows 10 version 2004 or later for capture exclusion

## License

MIT
