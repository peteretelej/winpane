# Input and Hit Testing

## Click-through model

All winpane surfaces start as topmost windows with `WS_EX_NOACTIVATE` (never steal focus) and `WS_EX_TOOLWINDOW` (no taskbar entry). Whether they intercept mouse input depends on their `WM_NCHITTEST` response:

- **Hud surfaces** always return `HTTRANSPARENT`. All mouse events pass through to the window below. There is no way to make a Hud interactive; use a Panel instead.
- **Panel surfaces** use a `HitTestMap` to decide per-pixel. Interactive element regions return `HTCLIENT` (intercepted). The drag region returns `HTCAPTION` (Windows handles the drag natively). Everything else returns `HTTRANSPARENT`.

## HitTestMap

The `HitTestMap` stores a list of physical-pixel rectangles, one per interactive element. It is rebuilt whenever the scene graph changes. On `WM_NCHITTEST`, the window procedure converts the screen-space hit point to client coordinates and checks each rectangle in reverse z-order (topmost element first). The first match determines the result.

The hit test map operates in physical pixels because `WM_NCHITTEST` provides physical-pixel coordinates. Element positions (in logical pixels) are scaled by the current DPI factor when building the map.

## Interactive elements

Any text, rect, or image element can be made interactive by setting `interactive: true`. This does two things:

1. The element's bounding box is added to the `HitTestMap`
2. Mouse events on that region generate `ElementClicked`, `ElementHovered`, and `ElementLeft` events

The `interactive` flag only has effect on Panel surfaces. On Hud surfaces, it is silently ignored because all input passes through.

## Event types

- `ElementClicked { surface_id, key }` - Left mouse button released on an interactive element
- `ElementHovered { surface_id, key }` - Mouse entered an interactive element's bounds
- `ElementLeft { surface_id, key }` - Mouse left an interactive element's bounds

Events include the surface ID and element key so you can identify which element was interacted with. The consumer polls events with `Context::poll_event()`.

## Hover tracking

When the mouse enters an interactive element, the engine calls `TrackMouseEvent` with `TME_LEAVE` to get notified when the mouse leaves the window. A `PanelState` per window tracks which element (if any) is currently hovered. When the mouse moves to a different element or leaves the window, `ElementLeft` is emitted for the old element and `ElementHovered` for the new one.

## Drag

Panels with `draggable: true` define a drag region at the top of the window, `drag_height` pixels tall. When `WM_NCHITTEST` hits this region (and no interactive element is above it), it returns `HTCAPTION`. Windows then handles the drag natively: the user can click and drag the panel around without any custom code.

The drag region check happens after the interactive element check. If an interactive button overlaps the drag region, the button wins.

**Cursor feedback:** When the cursor hovers over the drag region (`HTCAPTION`), `panel_wndproc` handles `WM_SETCURSOR` to show `IDC_SIZEALL` (the four-arrow move cursor). This is self-gating — if `draggable` is false, `HTCAPTION` is never returned so the cursor change never fires.

## Focus management

winpane never calls `SetForegroundWindow` or `SetFocus`. All windows use `WS_EX_NOACTIVATE`, and the window procedure returns `MA_NOACTIVATE` from `WM_MOUSEACTIVATE`. This means clicking on a Panel's interactive element does not steal focus from the user's current application.
