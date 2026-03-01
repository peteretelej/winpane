# Rust Guide

## Install

Add winpane to your `Cargo.toml`:

```sh
cargo add winpane
```

winpane uses `windows-rs` internally and only compiles on Windows. If you develop on macOS/Linux, you can still write code against the API; use `cargo fmt` and `cargo clippy` locally, and test on a Windows machine or CI.

## Hello world

```rust
use winpane::{Color, Context, HudConfig, Placement, RectElement, TextElement};

fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;
    let hud = ctx.create_hud(HudConfig {
        placement: Placement::Monitor { index: 0, anchor: winpane::Anchor::TopLeft, margin: 40 },
        width: 300, height: 100,
        ..Default::default()
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

    // Keep the process alive. The surface disappears when Context drops.
    loop { std::thread::sleep(std::time::Duration::from_secs(1)); }
}
```

`Context::new()` spawns the engine thread. `create_hud` blocks until the window is created and returns a `Hud` handle. Element operations (`set_rect`, `set_text`) are fire-and-forget. `show()` makes the window visible.

## Elements

Three element types are available on Hud and Panel surfaces:

**TextElement** - Rendered with DirectWrite. Supports font family, size, bold, italic.

```rust
hud.set_text("label", TextElement {
    text: "CPU: 42%".into(),
    x: 16.0, y: 50.0,
    font_size: 14.0,
    color: Color::rgba(100, 220, 160, 255),
    font_family: Some("Consolas".into()),
    bold: true,
    ..Default::default()
});
```

**RectElement** - Filled rectangle with optional rounded corners and border.

```rust
hud.set_rect("card", RectElement {
    x: 10.0, y: 10.0, width: 280.0, height: 80.0,
    fill: Color::rgba(30, 30, 45, 220),
    corner_radius: 6.0,
    border_color: Some(Color::rgba(80, 80, 120, 150)),
    border_width: 1.0,
    ..Default::default()
});
```

**ImageElement** - Rendered from premultiplied RGBA pixel data.

```rust
let pixels: Vec<u8> = load_my_image(); // RGBA8, premultiplied alpha
hud.set_image("icon", ImageElement {
    x: 10.0, y: 10.0, width: 32.0, height: 32.0,
    data: pixels,
    data_width: 32,
    data_height: 32,
    interactive: false,
});
```

Elements are identified by string keys. Setting an element with an existing key replaces it. Insertion order determines z-order (later elements draw on top). `remove("key")` deletes an element.

## Interactive panels

Panels support mouse input on elements with `interactive: true`:

```rust
use winpane::{Context, PanelConfig, Placement, RectElement, TextElement, Color, Event};

let ctx = Context::new()?;
let panel = ctx.create_panel(PanelConfig {
    placement: Placement::Position { x: 200, y: 200 }, width: 260, height: 100,
    draggable: true, drag_height: 30,
    position_key: Some("my_panel".into()),
    ..Default::default()
})?;

panel.set_rect("btn", RectElement {
    x: 20.0, y: 40.0, width: 220.0, height: 40.0,
    fill: Color::rgba(50, 80, 140, 200),
    corner_radius: 6.0,
    interactive: true,
    ..Default::default()
});
panel.show();

loop {
    while let Some(event) = ctx.poll_event() {
        match event {
            Event::ElementClicked { surface_id, ref key }
                if surface_id == panel.id() && key == "btn" =>
            {
                println!("Button clicked");
            }
            _ => {}
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(16));
}
```

Poll events regularly. The SDK does not use callbacks.

## Surface operations

All surfaces share these methods:

```rust
surface.show();
surface.hide();
surface.set_position(x, y);
surface.set_size(width, height);
surface.set_opacity(0.8);           // 0.0 to 1.0
surface.set_capture_excluded(true); // hide from screenshots (Win10 2004+)
surface.set_backdrop(Backdrop::Mica); // Win11 22H2+
surface.fade_in(300);               // fade in over 300ms
surface.fade_out(500);              // fade out over 500ms, then hide
surface.anchor_to(hwnd, Anchor::TopRight, (8, 0)); // track a window
surface.unanchor();
```

## Placement

Surfaces can be placed at explicit coordinates or relative to a monitor corner:

```rust
use winpane::{Anchor, Placement};

// Explicit position
Placement::Position { x: 100, y: 100 }

// Relative to monitor edge
Placement::Monitor { index: 0, anchor: Anchor::BottomRight, margin: 20 }
```

Use `ctx.monitors()` to query available monitors:

```rust
for m in ctx.monitors() {
    println!("{}x{} at ({},{}) dpi={} primary={}",
        m.width, m.height, m.x, m.y, m.dpi, m.is_primary);
}
```

## Position persistence

Set `position_key` on a config to save and restore the surface position across sessions:

```rust
let panel = ctx.create_panel(PanelConfig {
    placement: Placement::Monitor { index: 0, anchor: Anchor::BottomRight, margin: 20 },
    width: 200, height: 100,
    draggable: true, drag_height: 28,
    position_key: Some("my_widget".into()),
    ..Default::default()
})?;
```

Positions are stored in `%LOCALAPPDATA%/winpane/positions.json`. When a surface with a known key is created, its saved position is restored automatically.

The `SurfaceMoved` event is emitted whenever a surface moves:

```rust
if let Event::SurfaceMoved { surface_id, x, y } = event {
    println!("Surface {surface_id:?} moved to ({x}, {y})");
}
```

## Tray icons

```rust
use winpane::{Context, TrayConfig, MenuItem, Event};

let icon_data = vec![0x3C, 0x78, 0xDC, 0xFF].repeat(32 * 32);
let ctx = Context::new()?;
let tray = ctx.create_tray(TrayConfig {
    icon_rgba: icon_data,
    icon_width: 32, icon_height: 32,
    tooltip: "My App".into(),
})?;

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

Associate a Panel as the tray popup with `tray.set_popup(&panel)`. Left-clicking the tray icon toggles the panel's visibility.

## Custom draw

For procedural rendering beyond the scene graph:

```rust
use winpane::DrawOp;

hud.custom_draw(vec![
    DrawOp::FillRoundedRect {
        x: 10.0, y: 10.0, width: 100.0, height: 50.0,
        radius: 4.0,
        color: Color::rgba(80, 160, 255, 255),
    },
    DrawOp::DrawText {
        x: 20.0, y: 20.0,
        text: "Custom".into(),
        font_size: 14.0,
        color: Color::WHITE,
    },
]);
```

Custom draw is one-shot: the next scene graph change overwrites it. Only available in-process (Rust and C APIs).

## Cleanup

Surfaces are destroyed when their handles are dropped. `Context` is destroyed when it goes out of scope, which shuts down the engine thread and closes all windows. There is no explicit cleanup needed beyond normal Rust ownership.

## Next steps

- [Cookbook](../cookbook.md) - 10 recipes for common patterns
- [Design overview](../design.md) - Architecture and internals
- [Limitations](../limitations.md) - Known constraints
