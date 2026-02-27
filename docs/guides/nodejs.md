# Node.js Guide

## Install

```sh
npm install winpane
```

The package includes prebuilt native binaries for Windows x64. No build tools required.

## Hello world

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

// Keep the process alive
setInterval(() => {}, 1000);
```

`new WinPane()` spawns the engine thread. `createHud()` returns a numeric surface ID. Element and surface methods take this ID as the first argument.

## Colors

Colors are hex strings. The `#` prefix is optional.

| Format | Example | Description |
|--------|---------|-------------|
| `#rgb` | `"#f00"` | Shorthand, alpha 255 |
| `#rrggbb` | `"#ff0000"` | Full hex, alpha 255 |
| `#rrggbbaa` | `"#ff000080"` | With alpha (00 = transparent, ff = opaque) |

Default text color is white (`#ffffff`). Default rect fill is white.

## Elements

Three element types, set on Hud and Panel surfaces:

**Text:**

```js
wp.setText(surfaceId, "label", {
  text: "CPU: 42%",
  x: 16, y: 50,
  fontSize: 14,
  color: "#64dc9f",
  fontFamily: "Consolas", // optional
  bold: true,             // optional
  italic: false,          // optional
  interactive: false,     // optional, Panel only
});
```

**Rect:**

```js
wp.setRect(surfaceId, "card", {
  x: 10, y: 10, width: 280, height: 80,
  fill: "#1e1e2ddc",
  cornerRadius: 6,             // optional
  borderColor: "#505078aa",    // optional
  borderWidth: 1,              // optional
  interactive: false,          // optional
});
```

**Image:**

```js
wp.setImage(surfaceId, "icon", {
  path: "C:/icons/logo.png",   // local file path
  x: 10, y: 10,
  width: 32, height: 32,
  interactive: false,           // optional
});
```

Remove an element with `wp.removeElement(surfaceId, "key")`.

Elements are keyed by string. Setting the same key replaces the element. Insertion order is z-order.

## Interactive panels

```js
const panel = wp.createPanel({
  width: 260, height: 100,
  x: 200, y: 200,
  draggable: true,
  dragHeight: 30,
});

wp.setRect(panel, "btn", {
  x: 20, y: 40, width: 220, height: 40,
  fill: "#32508cc8",
  cornerRadius: 6,
  interactive: true,
});
wp.show(panel);

setInterval(() => {
  let event;
  while ((event = wp.pollEvent())) {
    if (event.eventType === "element_clicked" && event.key === "btn") {
      console.log("Button clicked");
    }
    if (event.eventType === "element_hovered" && event.key === "btn") {
      wp.setRect(panel, "btn", {
        x: 20, y: 40, width: 220, height: 40,
        fill: "#4664aadc",
        cornerRadius: 6,
        interactive: true,
      });
    }
    if (event.eventType === "element_left" && event.key === "btn") {
      wp.setRect(panel, "btn", {
        x: 20, y: 40, width: 220, height: 40,
        fill: "#32508cc8",
        cornerRadius: 6,
        interactive: true,
      });
    }
  }
}, 16);
```

Poll events with `wp.pollEvent()`. Returns an object or `null`. Event types: `element_clicked`, `element_hovered`, `element_left`, `tray_clicked`, `tray_menu_item_clicked`, `pip_source_closed`, `anchor_target_closed`, `device_recovered`.

## Surface control

```js
wp.show(id);
wp.hide(id);
wp.setPosition(id, 500, 300);
wp.setSize(id, 400, 200);
wp.setOpacity(id, 0.8);
wp.setCaptureExcluded(id, true);      // hide from screenshots
wp.setBackdrop(id, "mica");           // "mica", "acrylic", or "none"
wp.fadeIn(id, 300);                    // fade in over 300ms
wp.fadeOut(id, 500);                   // fade out, then hide
wp.anchorTo(id, targetHwnd, "top_right", 8, 0);
wp.unanchor(id);
```

Check backdrop support at runtime:

```js
if (wp.backdropSupported()) {
  wp.setBackdrop(id, "mica");
}
```

## Tray icons

```js
const tray = wp.createTray({ tooltip: "My App", iconPath: "icon.png" });
wp.setMenu(tray, [
  { id: 1, label: "Settings" },
  { id: 99, label: "Quit" },
]);

// Associate a panel as popup (left-click toggles)
const popup = wp.createPanel({ width: 200, height: 100 });
wp.setRect(popup, "bg", { x: 0, y: 0, width: 200, height: 100, fill: "#1e1e28f0" });
wp.setPopup(tray, popup);
```

If no `iconPath` is provided, a default 16x16 white icon is used.

## PiP (Picture-in-Picture)

```js
const pip = wp.createPip({
  sourceHwnd: 0x12345,  // HWND of the window to thumbnail
  width: 320, height: 240,
  x: 50, y: 50,
});
wp.show(pip);

// Optional: crop the source
wp.setSourceRegion(pip, { x: 0, y: 0, width: 800, height: 600 });
wp.clearSourceRegion(pip); // show full window again
```

PiP surfaces do not support scene graph elements (setText, setRect, setImage).

## Cleanup

```js
wp.destroy(surfaceId);  // destroy a specific surface or tray
wp.close();             // destroy all surfaces and shut down the engine
```

The engine shuts down when the `WinPane` instance is garbage collected, but calling `close()` explicitly is recommended.

## Next steps

- [Cookbook](../cookbook.md) - 10 recipes with Node.js equivalents
- [Design overview](../design.md) - Architecture and internals
- [Limitations](../limitations.md) - Known constraints
