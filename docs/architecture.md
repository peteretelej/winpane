# Architecture

## Overview

winpane is an out-of-process DirectComposition overlay SDK for Windows. It creates topmost companion surfaces (HUDs, panels, PiP thumbnails, tray icons) rendered via D3D11, Direct2D, and DirectWrite, composed by DWM through DirectComposition visual trees. All windows use `WS_EX_NOREDIRECTIONBITMAP` for GPU-native per-pixel transparency without legacy `UpdateLayeredWindow` paths.

The SDK is consumed directly from Rust, or through C ABI bindings, a Node.js napi-rs addon, or a JSON-RPC CLI host for any language.

## Crate map

```
winpane-core (internal)
  |
  v
winpane (public Rust API)
  |
  +---> winpane-ffi  (cdylib, C ABI via cbindgen)
  +---> winpane-host (CLI binary, JSON-RPC over stdin/stdout)
  +---> bindings/node (napi-rs addon for Node.js/Bun)
```

All external crates depend on `winpane`, never on `winpane-core` directly. The core crate contains Win32, DirectComposition, and Direct2D implementation details. The `winpane` crate provides the ergonomic public API with typed surface handles.

## Threading model

The engine runs on a dedicated thread that owns the Win32 message loop, all HWNDs, and all GPU resources. Consumer threads communicate with the engine via an MPSC command channel. `PostMessageW` to a message-only control window wakes the engine's `GetMessageW` loop to drain pending commands.

Events flow back to the consumer through a separate MPSC receiver, polled with `Context::poll_event()`. There are no callbacks; the consumer drives their own event loop.

Several thread-local queues bridge Win32 window procedures to the engine loop: `PENDING_DPI_CHANGES` for `WM_DPICHANGED`, `PENDING_TRAY_EVENTS` for tray icon notifications, and `PENDING_FADE_COMPLETIONS` for animation timers. The engine drains these after each message dispatch.

## Rendering pipeline

The GPU stack is: D3D11 device (hardware, WARP fallback) -> DXGI swap chain (composition mode, `FLIP_SEQUENTIAL`, premultiplied alpha) -> Direct2D device context -> DirectComposition visual bound to HWND.

Each render cycle: `GetBuffer` -> `CreateBitmapFromDxgiSurface` -> `BeginDraw` -> `Clear(transparent)` -> render elements back-to-front -> `EndDraw` -> `Present` -> `Commit`. The swap chain uses `DXGI_ALPHA_MODE_PREMULTIPLIED` and two buffers in BGRA format.

DirectComposition handles compositing the swap chain output into the desktop, giving per-pixel transparency without `WS_EX_LAYERED`.

## Surface lifecycle

Surfaces are created synchronously via a reply channel: the consumer sends a `Create*` command and blocks on a oneshot receiver until the engine thread creates the HWND, swap chain, and renderer, then replies with a `SurfaceId`. After creation, all operations (set elements, show/hide, reposition) are fire-and-forget commands sent through the MPSC channel.

Surfaces are hidden by default. `show()` makes the window visible; `hide()` hides it. `DestroySurface` destroys the window and frees GPU resources. Surface handles implement `Drop` to send a destroy command automatically.

## Scene graph

Each surface maintains a `SceneGraph` backed by an `IndexMap<String, Element>`. Insertion order determines z-order (later elements draw on top). Element types are `Text`, `Rect`, and `Image`, addressed by string keys.

A dirty flag tracks changes. The renderer checks `take_dirty()` each frame to decide if a redraw is needed, avoiding unnecessary GPU work when elements haven't changed. Custom draw operations (`DrawOp`) overwrite the scene rendering for one frame until the next scene graph change restores normal rendering.

## DPI handling

winpane uses per-monitor DPI awareness v2. Each surface tracks its current DPI scale factor. When `WM_DPICHANGED` arrives, the window procedure queues a `DpiChangeEvent` into the thread-local `PENDING_DPI_CHANGES`. The engine processes these by resizing the swap chain, updating the DPI scale, and marking the scene dirty for re-render.

All public API coordinates are in logical pixels. Anchor offsets are scaled by the target monitor's DPI factor to maintain consistent visual distances across mixed-DPI setups.

## GPU device loss recovery

Direct3D device loss (`DXGI_ERROR_DEVICE_REMOVED` or `DXGI_ERROR_DEVICE_RESET`) is detected during swap chain `Present`. Recovery destroys all GPU resources, creates a new D3D11 device (with WARP fallback), and rebuilds the `SurfaceRenderer` for every active surface. The scene graph is preserved in memory, so all elements are re-rendered without consumer intervention. A `DeviceRecovered` event notifies the consumer after recovery completes.

## Input and hit testing

Panel surfaces use `WM_NCHITTEST` with a `HitTestMap` to provide selective click-through. The hit test map stores physical-pixel regions for each interactive element. Mouse clicks within an interactive element's bounds return `HTCLIENT` (intercepted); clicks elsewhere return `HTTRANSPARENT` (pass through to windows below).

Hover tracking uses `TrackMouseEvent` with `TME_LEAVE` to detect mouse enter/leave on interactive elements. The engine emits `ElementHovered` and `ElementLeft` events. The drag region at the top of a panel returns `HTCAPTION` from hit testing, letting Windows handle the drag natively.

HUD surfaces always return `HTTRANSPARENT` for all mouse input.

## Window monitoring and anchoring

Anchored surfaces track a target window using `SetWinEventHook` for `EVENT_OBJECT_LOCATIONCHANGE`. When the target moves, the engine recalculates the anchor position based on the anchor corner (TopLeft, TopRight, BottomLeft, BottomRight) and the configured offset, then repositions the surface.

Minimization is tracked via `EVENT_OBJECT_HIDE` / `EVENT_OBJECT_SHOW`. When the target minimizes, the anchored surface hides and records its prior visibility. When the target restores, the surface is shown again if it was previously visible. If the target window is destroyed, an `AnchorTargetClosed` event is emitted.
