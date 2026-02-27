# Surface Cookbook

Self-contained recipes for common winpane patterns. Each recipe shows Rust code with one-line equivalents for Node.js and JSON-RPC.

## 1. Simple HUD overlay

Create a floating stats display with a background rect and text.

```rust
use winpane::{Color, Context, HudConfig, RectElement, TextElement};

let ctx = Context::new()?;
let hud = ctx.create_hud(HudConfig {
    x: 100, y: 100, width: 300, height: 120,
})?;

hud.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 300.0, height: 120.0,
    fill: Color::rgba(20, 20, 30, 200),
    corner_radius: 8.0,
    ..Default::default()
});

hud.set_text("title", TextElement {
    text: "Status".into(),
    x: 16.0, y: 12.0, font_size: 18.0,
    color: Color::WHITE, bold: true,
    ..Default::default()
});

hud.set_text("value", TextElement {
    text: "CPU: 42%".into(),
    x: 16.0, y: 50.0, font_size: 14.0,
    color: Color::rgba(100, 220, 160, 255),
    ..Default::default()
});

hud.show();
```

```js
// Node.js: const wp = new WinPane(); const id = wp.createHud({ width: 300, height: 120, x: 100, y: 100 });
```
```json
// JSON-RPC: {"jsonrpc":"2.0","method":"create_hud","params":{"x":100,"y":100,"width":300,"height":120},"id":1}
```

## 2. Interactive panel with buttons

Create a panel with clickable elements and event handling.

```rust
use winpane::{Color, Context, Event, PanelConfig, RectElement, TextElement};

let ctx = Context::new()?;
let panel = ctx.create_panel(PanelConfig {
    x: 200, y: 200, width: 260, height: 160,
    draggable: true, drag_height: 32,
})?;

// Background
panel.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 260.0, height: 160.0,
    fill: Color::rgba(25, 25, 35, 230), corner_radius: 8.0,
    ..Default::default()
});

// Clickable button
panel.set_rect("btn", RectElement {
    x: 20.0, y: 50.0, width: 220.0, height: 40.0,
    fill: Color::rgba(50, 80, 140, 200), corner_radius: 6.0,
    interactive: true,
    ..Default::default()
});
panel.set_text("btn_label", TextElement {
    text: "Click Me".into(),
    x: 90.0, y: 60.0, font_size: 14.0,
    color: Color::WHITE,
    ..Default::default()
});

panel.show();

let panel_id = panel.id();
loop {
    while let Some(event) = ctx.poll_event() {
        match event {
            Event::ElementClicked { surface_id, ref key }
                if surface_id == panel_id && key == "btn" =>
            {
                println!("Button clicked!");
            }
            Event::ElementHovered { surface_id, ref key }
                if surface_id == panel_id && key == "btn" =>
            {
                panel.set_rect("btn", RectElement {
                    x: 20.0, y: 50.0, width: 220.0, height: 40.0,
                    fill: Color::rgba(70, 100, 170, 220), corner_radius: 6.0,
                    interactive: true,
                    ..Default::default()
                });
            }
            Event::ElementLeft { surface_id, ref key }
                if surface_id == panel_id && key == "btn" =>
            {
                panel.set_rect("btn", RectElement {
                    x: 20.0, y: 50.0, width: 220.0, height: 40.0,
                    fill: Color::rgba(50, 80, 140, 200), corner_radius: 6.0,
                    interactive: true,
                    ..Default::default()
                });
            }
            _ => {}
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(16));
}
```

```js
// Node.js: const id = wp.createPanel({ width: 260, height: 160, x: 200, y: 200, draggable: true, dragHeight: 32 });
```
```json
// JSON-RPC: {"jsonrpc":"2.0","method":"create_panel","params":{"x":200,"y":200,"width":260,"height":160,"draggable":true,"drag_height":32},"id":1}
```

## 3. System tray with popup

Create a tray icon that toggles a popup panel on left-click.

```rust
use winpane::{Color, Context, Event, MenuItem, PanelConfig, RectElement, TextElement, TrayConfig};

let ctx = Context::new()?;

// Generate a 32x32 colored icon (RGBA)
let icon_size = 32u32;
let icon_data = vec![0x3C, 0x78, 0xDC, 0xFF].repeat((icon_size * icon_size) as usize);

let tray = ctx.create_tray(TrayConfig {
    icon_rgba: icon_data,
    icon_width: icon_size,
    icon_height: icon_size,
    tooltip: "My App".into(),
})?;

// Create a popup panel
let popup = ctx.create_panel(PanelConfig {
    x: 0, y: 0, width: 200, height: 100,
    draggable: false, drag_height: 0,
})?;
popup.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 200.0, height: 100.0,
    fill: Color::rgba(30, 30, 40, 240), corner_radius: 8.0,
    ..Default::default()
});
popup.set_text("msg", TextElement {
    text: "Hello from tray!".into(),
    x: 16.0, y: 16.0, font_size: 14.0,
    color: Color::WHITE,
    ..Default::default()
});

// Associate popup (left-click toggles visibility)
tray.set_popup(&popup);

// Right-click context menu
tray.set_menu(vec![
    MenuItem { id: 1, label: "Settings".into(), enabled: true },
    MenuItem { id: 99, label: "Quit".into(), enabled: true },
]);

loop {
    while let Some(event) = ctx.poll_event() {
        if let Event::TrayMenuItemClicked { id: 99 } = event {
            return Ok(());
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(16));
}
```

```js
// Node.js: const tid = wp.createTray({ tooltip: "My App" }); wp.setPopup(tid, panelId);
```
```json
// JSON-RPC: {"jsonrpc":"2.0","method":"create_tray","params":{"tooltip":"My App"},"id":1}
```

## 4. PiP viewer

Show a live DWM thumbnail of another window.

```rust
use winpane::{Context, Event, PipConfig};

let source_hwnd: isize = 0x12345; // Target window handle

let ctx = Context::new()?;
let pip = ctx.create_pip(PipConfig {
    source_hwnd,
    x: 50, y: 50, width: 400, height: 300,
})?;

pip.set_opacity(0.95);
pip.show();

loop {
    if let Some(Event::PipSourceClosed { .. }) = ctx.poll_event() {
        println!("Source window closed.");
        break;
    }
    std::thread::sleep(std::time::Duration::from_millis(16));
}
```

```js
// Node.js: const id = wp.createPip({ sourceHwnd: 0x12345, width: 400, height: 300, x: 50, y: 50 });
```
```json
// JSON-RPC: {"jsonrpc":"2.0","method":"create_pip","params":{"source_hwnd":74565,"x":50,"y":50,"width":400,"height":300},"id":1}
```

## 5. Anchored companion

Attach a surface to a corner of another window so it follows movement.

```rust
use winpane::{Anchor, Color, Context, Event, PanelConfig, RectElement, TextElement};

let target_hwnd: isize = 0x12345; // Target window handle

let ctx = Context::new()?;
let panel = ctx.create_panel(PanelConfig {
    x: 0, y: 0, width: 180, height: 100,
    draggable: false, drag_height: 0,
})?;

panel.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 180.0, height: 100.0,
    fill: Color::rgba(20, 20, 35, 230), corner_radius: 8.0,
    ..Default::default()
});
panel.set_text("label", TextElement {
    text: "Companion".into(),
    x: 12.0, y: 12.0, font_size: 14.0,
    color: Color::WHITE, bold: true,
    ..Default::default()
});

// Anchor to top-right with 8px horizontal offset
panel.anchor_to(target_hwnd, Anchor::TopRight, (8, 0));
panel.show();

loop {
    if let Some(Event::AnchorTargetClosed { .. }) = ctx.poll_event() {
        break;
    }
    std::thread::sleep(std::time::Duration::from_millis(16));
}
```

```js
// Node.js: wp.anchorTo(surfaceId, targetHwnd, "top_right", 8, 0);
```
```json
// JSON-RPC: {"jsonrpc":"2.0","method":"anchor_to","params":{"surface_id":"s1","target_hwnd":74565,"anchor":"top_right","offset_x":8,"offset_y":0},"id":5}
```

## 6. Backdrop effects

Apply Mica or Acrylic backdrop to a surface (Windows 11 22H2+).

```rust
use winpane::{Backdrop, Color, Context, HudConfig, RectElement, TextElement};

let ctx = Context::new()?;
let hud = ctx.create_hud(HudConfig {
    x: 100, y: 100, width: 300, height: 150,
})?;

// Use a semi-transparent background to let the backdrop show through
hud.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 300.0, height: 150.0,
    fill: Color::rgba(0, 0, 0, 40), corner_radius: 12.0,
    ..Default::default()
});

hud.set_text("title", TextElement {
    text: "Mica Surface".into(),
    x: 16.0, y: 16.0, font_size: 18.0,
    color: Color::WHITE, bold: true,
    ..Default::default()
});

// Check support at runtime
if winpane::backdrop_supported() {
    hud.set_backdrop(Backdrop::Mica);
}

hud.show();
```

```js
// Node.js: if (wp.backdropSupported()) { wp.setBackdrop(surfaceId, "mica"); }
```
```json
// JSON-RPC: {"jsonrpc":"2.0","method":"set_backdrop","params":{"surface_id":"s1","backdrop":"mica"},"id":10}
```

## 7. Fade transitions

Fade a surface in on start and out on dismiss.

```rust
use winpane::{Color, Context, HudConfig, RectElement, TextElement};

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
    text: "Notification".into(),
    x: 16.0, y: 16.0, font_size: 16.0,
    color: Color::WHITE,
    ..Default::default()
});

// Fade in over 300ms (shows the surface automatically)
hud.fade_in(300);

// Later: fade out over 500ms (hides the surface when complete)
std::thread::sleep(std::time::Duration::from_secs(3));
hud.fade_out(500);
```

```js
// Node.js: wp.fadeIn(surfaceId, 300); /* later */ wp.fadeOut(surfaceId, 500);
```
```json
// JSON-RPC: {"jsonrpc":"2.0","method":"fade_in","params":{"surface_id":"s1","duration_ms":300},"id":11}
```

## 8. Capture-excluded overlay

Create a HUD that is invisible in screenshots and screen recordings.

```rust
use winpane::{Color, Context, HudConfig, RectElement, TextElement};

let ctx = Context::new()?;
let hud = ctx.create_hud(HudConfig {
    x: 100, y: 100, width: 300, height: 80,
})?;

hud.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 300.0, height: 80.0,
    fill: Color::rgba(30, 10, 10, 230), corner_radius: 8.0,
    ..Default::default()
});

hud.set_text("label", TextElement {
    text: "Private overlay".into(),
    x: 16.0, y: 16.0, font_size: 16.0,
    color: Color::rgba(255, 80, 80, 255),
    ..Default::default()
});

// Exclude from screenshots and screen sharing (Win10 2004+)
hud.set_capture_excluded(true);
hud.show();
```

```js
// Node.js: wp.setCaptureExcluded(surfaceId, true);
```
```json
// JSON-RPC: {"jsonrpc":"2.0","method":"set_capture_excluded","params":{"surface_id":"s1","excluded":true},"id":5}
```

## 9. Custom draw

Use `DrawOp` for procedural rendering beyond the retained-mode scene graph.

```rust
use winpane::{Color, Context, DrawOp, HudConfig, RectElement};

let ctx = Context::new()?;
let hud = ctx.create_hud(HudConfig {
    x: 200, y: 200, width: 400, height: 300,
})?;

// Retained-mode background
hud.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 400.0, height: 300.0,
    fill: Color::rgba(15, 15, 25, 220), corner_radius: 8.0,
    ..Default::default()
});

hud.show();
std::thread::sleep(std::time::Duration::from_millis(100));

// Custom draw: bar chart
let ops = vec![
    DrawOp::DrawText {
        x: 20.0, y: 15.0,
        text: "Chart".into(), font_size: 18.0,
        color: Color::WHITE,
    },
    DrawOp::FillRoundedRect {
        x: 40.0, y: 60.0, width: 60.0, height: 180.0,
        radius: 4.0,
        color: Color::rgba(80, 160, 255, 255),
    },
    DrawOp::FillRoundedRect {
        x: 120.0, y: 120.0, width: 60.0, height: 120.0,
        radius: 4.0,
        color: Color::rgba(100, 220, 160, 255),
    },
    DrawOp::DrawLine {
        x1: 30.0, y1: 240.0, x2: 370.0, y2: 240.0,
        color: Color::rgba(80, 80, 120, 200),
        stroke_width: 1.0,
    },
];
hud.custom_draw(ops);
```

Custom draw is only available through the Rust and C APIs. It is not exposed over JSON-RPC or Node.js because it requires in-process GPU access.

## 10. Multi-surface dashboard

Combine a tray icon, popup panel, and anchored companion into a complete application.

```rust
use winpane::{
    Anchor, Color, Context, Event, MenuItem, PanelConfig, RectElement, TextElement, TrayConfig,
};

let ctx = Context::new()?;

// 1. Tray icon
let icon_data = vec![0x3C, 0x78, 0xDC, 0xFF].repeat(32 * 32);
let tray = ctx.create_tray(TrayConfig {
    icon_rgba: icon_data,
    icon_width: 32, icon_height: 32,
    tooltip: "Dashboard".into(),
})?;

// 2. Popup panel (toggled by tray left-click)
let popup = ctx.create_panel(PanelConfig {
    x: 0, y: 0, width: 240, height: 140,
    draggable: false, drag_height: 0,
})?;
popup.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 240.0, height: 140.0,
    fill: Color::rgba(30, 30, 40, 240), corner_radius: 8.0,
    ..Default::default()
});
popup.set_text("title", TextElement {
    text: "Dashboard".into(),
    x: 16.0, y: 12.0, font_size: 16.0,
    color: Color::WHITE, bold: true,
    ..Default::default()
});
tray.set_popup(&popup);
tray.set_menu(vec![
    MenuItem { id: 99, label: "Quit".into(), enabled: true },
]);

// 3. Anchored companion to another window
let target_hwnd: isize = 0x12345;
let companion = ctx.create_panel(PanelConfig {
    x: 0, y: 0, width: 160, height: 80,
    draggable: false, drag_height: 0,
})?;
companion.set_rect("bg", RectElement {
    x: 0.0, y: 0.0, width: 160.0, height: 80.0,
    fill: Color::rgba(20, 20, 35, 230), corner_radius: 8.0,
    ..Default::default()
});
companion.set_text("info", TextElement {
    text: "Tracking...".into(),
    x: 12.0, y: 12.0, font_size: 12.0,
    color: Color::rgba(180, 180, 200, 255),
    ..Default::default()
});
companion.anchor_to(target_hwnd, Anchor::TopRight, (8, 0));
companion.show();

// Event loop
loop {
    while let Some(event) = ctx.poll_event() {
        match event {
            Event::TrayMenuItemClicked { id: 99 } => return Ok(()),
            Event::AnchorTargetClosed { .. } => companion.hide(),
            _ => {}
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(16));
}
```

```js
// Node.js: Combine createTray, createPanel, setPopup, anchorTo for the same pattern.
```
```json
// JSON-RPC: Chain create_tray, create_panel, set_popup, anchor_to calls sequentially.
```
