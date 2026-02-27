# TypeScript / JavaScript Guide

The `winpane` npm package is a native Node.js addon built with napi-rs. It works with Node.js, Bun, and Electron. TypeScript type definitions are generated automatically by napi-rs.

For Deno or environments where native addons are not available, use the JSON-RPC host as a subprocess instead (see the [alternative approach](#alternative-json-rpc-host) section).

## Install

```sh
npm install winpane
```

Prebuilt binaries are included for Windows x64 and ARM64. No build tools needed.

## Hello world

```typescript
import { WinPane } from "winpane";

const wp = new WinPane();
const hud = wp.createHud({ width: 300, height: 100, x: 100, y: 100 });

wp.setRect(hud, "bg", {
  x: 0, y: 0, width: 300, height: 100,
  fill: "#14141ec8", cornerRadius: 8,
});
wp.setText(hud, "msg", {
  text: "Hello from TypeScript",
  x: 16, y: 16, fontSize: 18,
});
wp.show(hud);

// Keep the process alive
setInterval(() => {}, 1000);
```

## API types

Surface creation methods return a numeric ID. All subsequent calls take this ID as the first argument.

```typescript
// Surface creation
wp.createHud(options: HudOptions): number
wp.createPanel(options: PanelOptions): number
wp.createPip(options: PipOptions): number
wp.createTray(options: TrayOptions): number

// Elements (Hud and Panel only)
wp.setText(surfaceId: number, key: string, options: TextOptions): void
wp.setRect(surfaceId: number, key: string, options: RectOptions): void
wp.setImage(surfaceId: number, key: string, options: ImageOptions): void
wp.removeElement(surfaceId: number, key: string): void

// Surface control
wp.show(surfaceId: number): void
wp.hide(surfaceId: number): void
wp.setPosition(surfaceId: number, x: number, y: number): void
wp.setSize(surfaceId: number, width: number, height: number): void
wp.setOpacity(surfaceId: number, opacity: number): void
wp.fadeIn(surfaceId: number, durationMs: number): void
wp.fadeOut(surfaceId: number, durationMs: number): void
wp.setCaptureExcluded(surfaceId: number, excluded: boolean): void
wp.setBackdrop(surfaceId: number, backdrop: "none" | "mica" | "acrylic"): void
wp.backdropSupported(): boolean

// Anchoring
wp.anchorTo(surfaceId: number, targetHwnd: number, anchor: string, offsetX: number, offsetY: number): void
wp.unanchor(surfaceId: number): void

// PiP-specific
wp.setSourceRegion(surfaceId: number, options: SourceRegionOptions): void
wp.clearSourceRegion(surfaceId: number): void

// Tray
wp.setTooltip(trayId: number, tooltip: string): void
wp.setTrayIcon(trayId: number, iconPath: string): void
wp.setPopup(trayId: number, panelSurfaceId: number): void
wp.setMenu(trayId: number, items: MenuItemOptions[]): void

// Events
wp.pollEvent(): WinPaneEvent | null

// Lifecycle
wp.destroy(id: number): void
wp.close(): void
```

## Option types

```typescript
interface HudOptions {
  width: number;
  height: number;
  x?: number;  // default: 0
  y?: number;  // default: 0
}

interface PanelOptions {
  width: number;
  height: number;
  x?: number;
  y?: number;
  draggable?: boolean;  // default: false
  dragHeight?: number;   // default: 0
}

interface PipOptions {
  sourceHwnd: number;    // HWND as a 64-bit integer
  width: number;
  height: number;
  x?: number;
  y?: number;
}

interface TrayOptions {
  iconPath?: string;     // path to PNG/JPEG/BMP, default: white 16x16
  tooltip?: string;
}

interface TextOptions {
  text: string;
  x: number;
  y: number;
  fontSize: number;
  color?: string;        // hex, default: "#ffffff"
  fontFamily?: string;
  bold?: boolean;
  italic?: boolean;
  interactive?: boolean; // Panel only
}

interface RectOptions {
  x: number;
  y: number;
  width: number;
  height: number;
  fill?: string;           // hex, default: "#ffffff"
  cornerRadius?: number;
  borderColor?: string;    // hex
  borderWidth?: number;
  interactive?: boolean;
}

interface ImageOptions {
  path: string;            // local file path
  x: number;
  y: number;
  width: number;
  height: number;
  interactive?: boolean;
}

interface MenuItemOptions {
  id: number;
  label: string;
  enabled?: boolean;       // default: true
}

interface SourceRegionOptions {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface WinPaneEvent {
  eventType: string;       // "element_clicked", "element_hovered", etc.
  surfaceId?: number;
  key?: string;
  button?: string;         // "left", "right", "middle"
  itemId?: number;
}
```

## Colors

Hex strings with optional `#` prefix:

```typescript
"#f00"       // shorthand, alpha 255
"#ff0000"    // full hex, alpha 255
"#ff000080"  // with alpha (00 = transparent, ff = opaque)
"ff0000"     // # prefix is optional
```

## Interactive panels with events

```typescript
const panel = wp.createPanel({
  width: 260, height: 120,
  x: 200, y: 200,
  draggable: true,
  dragHeight: 30,
});

wp.setRect(panel, "bg", {
  x: 0, y: 0, width: 260, height: 120,
  fill: "#191923e6", cornerRadius: 8,
});

wp.setRect(panel, "btn", {
  x: 20, y: 50, width: 220, height: 40,
  fill: "#32508cc8", cornerRadius: 6,
  interactive: true,
});

wp.setText(panel, "btn_label", {
  text: "Click Me",
  x: 90, y: 60, fontSize: 14,
});

wp.show(panel);

// Event loop
setInterval(() => {
  let event: WinPaneEvent | null;
  while ((event = wp.pollEvent()) !== null) {
    switch (event.eventType) {
      case "element_clicked":
        if (event.key === "btn") {
          console.log("Button clicked");
        }
        break;
      case "element_hovered":
        if (event.key === "btn") {
          wp.setRect(panel, "btn", {
            x: 20, y: 50, width: 220, height: 40,
            fill: "#4664aadc", cornerRadius: 6,
            interactive: true,
          });
        }
        break;
      case "element_left":
        if (event.key === "btn") {
          wp.setRect(panel, "btn", {
            x: 20, y: 50, width: 220, height: 40,
            fill: "#32508cc8", cornerRadius: 6,
            interactive: true,
          });
        }
        break;
    }
  }
}, 16);
```

## Tray with popup

```typescript
const tray = wp.createTray({ tooltip: "My App", iconPath: "icon.png" });

const popup = wp.createPanel({ width: 200, height: 100 });
wp.setRect(popup, "bg", {
  x: 0, y: 0, width: 200, height: 100,
  fill: "#1e1e28f0", cornerRadius: 8,
});
wp.setText(popup, "msg", {
  text: "Hello from tray",
  x: 16, y: 16, fontSize: 14,
});

wp.setPopup(tray, popup);
wp.setMenu(tray, [
  { id: 1, label: "Settings" },
  { id: 99, label: "Quit" },
]);

setInterval(() => {
  let event;
  while ((event = wp.pollEvent()) !== null) {
    if (event.eventType === "tray_menu_item_clicked" && event.itemId === 99) {
      wp.close();
      process.exit(0);
    }
  }
}, 16);
```

## Backdrop effects

```typescript
if (wp.backdropSupported()) {
  wp.setBackdrop(hud, "mica");
}
```

Use semi-transparent fills (low alpha) on background rects so the Mica/Acrylic effect shows through.

## Electron

The native addon works in Electron's main process. Create overlays for your app from the main process, not the renderer.

```typescript
// main.ts (Electron main process)
import { WinPane } from "winpane";
import { app, BrowserWindow } from "electron";

app.whenReady().then(() => {
  const win = new BrowserWindow({ width: 800, height: 600 });

  const wp = new WinPane();
  const companion = wp.createPanel({ width: 200, height: 80 });
  wp.setRect(companion, "bg", {
    x: 0, y: 0, width: 200, height: 80,
    fill: "#14141ec8", cornerRadius: 8,
  });
  wp.setText(companion, "label", {
    text: "Companion panel",
    x: 12, y: 12, fontSize: 13,
  });

  // Anchor to the Electron window
  const hwnd = win.getNativeWindowHandle().readInt32LE(0);
  wp.anchorTo(companion, hwnd, "top_right", 8, 0);
  wp.show(companion);
});
```

## Alternative: JSON-RPC host

For Deno, Bun without native addon support, or any JS runtime that cannot load napi modules, spawn `winpane-host` as a subprocess:

```typescript
import { spawn } from "child_process";
import { createInterface } from "readline";

const proc = spawn("winpane-host", [], {
  stdio: ["pipe", "pipe", "inherit"],
});

const rl = createInterface({ input: proc.stdout! });

let nextId = 1;
const pending = new Map<number, (result: any) => void>();

rl.on("line", (line) => {
  const msg = JSON.parse(line);
  if ("id" in msg && pending.has(msg.id)) {
    pending.get(msg.id)!(msg);
    pending.delete(msg.id);
  } else if (msg.method === "event") {
    console.log("Event:", msg.params);
  }
});

function rpc(method: string, params: Record<string, any>): Promise<any> {
  const id = nextId++;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    proc.stdin!.write(JSON.stringify({ jsonrpc: "2.0", method, params, id }) + "\n");
  });
}

// Usage
const { result } = await rpc("create_hud", { x: 100, y: 100, width: 300, height: 100 });
await rpc("set_text", {
  surface_id: result.surface_id,
  key: "msg", text: "Hello", x: 16, y: 16, font_size: 18,
});
await rpc("show", { surface_id: result.surface_id });
```

See the [protocol reference](../protocol.md) for the full method list.

## Next steps

- [Node.js guide](nodejs.md) - Same API, more detail on each method
- [Cookbook](../cookbook.md) - 10 recipes with Node.js equivalents
- [Design overview](../design.md) - Architecture and internals
- [Limitations](../limitations.md) - Known constraints
