# Surface Types

## Overview

winpane provides four surface types. Each maps to a Win32 window with specific style flags and behavior.

| Type | Scene graph | Input | DWM thumbnail | Tray icon |
|------|-------------|-------|---------------|-----------|
| Hud | Yes | Click-through | No | No |
| Panel | Yes | Selective | No | No |
| Pip | No | Click-through | Yes | No |
| Tray | No | N/A | No | Yes |

## Lifecycle

All surfaces follow the same lifecycle:

1. **Create** - Consumer sends a create command; engine creates the HWND, swap chain, and renderer. Returns a typed handle (`Hud`, `Panel`, `Pip`, or `Tray`). Creation is synchronous (blocks on a reply channel).
2. **Configure** - Add elements, set position/size/opacity, anchor, configure backdrop. All operations are fire-and-forget commands.
3. **Show** - `show()` makes the window visible. Surfaces are hidden by default after creation.
4. **Update** - Modify elements, reposition, resize. Changes are applied on the next engine loop iteration.
5. **Destroy** - Either call `destroy()` explicitly or let the handle drop. Drop sends a destroy command automatically. The window is closed and GPU resources are freed.

## Hud

Click-through overlay for passive information display.

**Win32 recipe:** `WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`. The window procedure returns `HTTRANSPARENT` for all `WM_NCHITTEST` messages, making the entire surface click-through.

**Config:**

```rust
HudConfig {
    placement: Placement, // where to place the window
    width: u32,           // initial width
    height: u32,          // initial height
    position_key: Option<String>, // persist position across sessions
}
```

**Operations:** set_text, set_rect, set_image, remove, show, hide, set_position, set_size, set_opacity, custom_draw, anchor_to, unanchor, set_capture_excluded, set_backdrop, fade_in, fade_out.

## Panel

Interactive surface with selective click-through.

**Win32 recipe:** Same base styles as Hud. The difference is in `WM_NCHITTEST` handling: the window procedure consults a `HitTestMap` to determine whether each mouse position hits an interactive element (`HTCLIENT`), a drag region (`HTCAPTION`), or empty space (`HTTRANSPARENT`).

**Config:**

```rust
PanelConfig {
    placement: Placement,
    width: u32,
    height: u32,
    draggable: bool,  // enable drag by title region
    drag_height: u32, // height of drag region from top (logical pixels)
    position_key: Option<String>, // persist position across sessions
}
```

**Operations:** Same as Hud, plus the `id()` method for matching events to surfaces.

**Events:** `ElementClicked`, `ElementHovered`, `ElementLeft` - emitted when the user interacts with elements that have `interactive: true`.

## Pip

Picture-in-Picture surface showing a live DWM thumbnail of another window.

**Win32 recipe:** Same base styles. Uses `DwmRegisterThumbnail` to create a live thumbnail of the source window, sized to fit the Pip surface. The thumbnail updates automatically as the source window repaints.

**Config:**

```rust
PipConfig {
    source_hwnd: isize, // HWND of the source window
    placement: Placement,
    width: u32,
    height: u32,
    position_key: Option<String>,
}
```

**Operations:** show, hide, set_position, set_size, set_opacity, set_source_region, clear_source_region, anchor_to, unanchor, set_capture_excluded, set_backdrop, fade_in, fade_out. Does not support scene graph operations (set_text, set_rect, set_image).

**Events:** `PipSourceClosed` - emitted when the source window is destroyed.

## Tray

System tray icon with popup panel and context menu.

**Implementation:** Uses `Shell_NotifyIconW` to create the tray icon. The icon image is created from RGBA pixel data via `CreateIconIndirect`. Left-click toggles a popup panel's visibility (if one is associated). Right-click shows a native context menu via `TrackPopupMenu`.

**Config:**

```rust
TrayConfig {
    icon_rgba: Vec<u8>, // RGBA8 pixel data for the icon
    icon_width: u32,
    icon_height: u32,
    tooltip: String,    // max 127 chars
}
```

**Operations:** set_tooltip, set_icon, set_popup (associates a Panel), set_menu (sets right-click menu items).

**Events:** `TrayClicked` (with mouse button), `TrayMenuItemClicked` (with menu item ID).

## Common operations

These apply to all surface types (except where noted):

- **Anchoring** - `anchor_to(target_hwnd, anchor, offset)` attaches the surface to a corner of another window. The surface follows the target as it moves. `unanchor()` detaches. Emits `AnchorTargetClosed` if the target is destroyed.
- **Capture exclusion** - `set_capture_excluded(true)` hides the surface from screenshots and screen sharing via `SetWindowDisplayAffinity`. Requires Windows 10 2004+.
- **Backdrop** - `set_backdrop(Mica)` or `set_backdrop(Acrylic)` applies a DWM backdrop effect. Requires Windows 11 22H2+. Use semi-transparent background rects to let the effect show through.
- **Fade animations** - `fade_in(ms)` shows the surface and animates opacity 0 to 1. `fade_out(ms)` animates 1 to 0 and hides. Uses DirectComposition opacity animations.
- **Position persistence** - Setting `position_key` on a config saves the surface's last position to `%LOCALAPPDATA%/winpane/positions.json`. On next launch with the same key, the surface restores to its saved position.
- **Position query** - `get_position()` returns the current `(x, y)` screen coordinates of the surface.

## Events

In addition to surface-type-specific events:

- **`SurfaceMoved { surface_id, x, y }`** - Emitted when a surface is moved (e.g., dragged by the user). Includes the new screen coordinates.
