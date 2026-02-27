# Rendering Pipeline

## GPU stack

The rendering chain is:

```
D3D11 Device (hardware, WARP fallback)
  -> DXGI Swap Chain (FLIP_SEQUENTIAL, premultiplied alpha, composition mode)
    -> Direct2D Device Context (created from DXGI surface)
      -> DirectComposition Visual (bound to HWND)
```

Each surface gets its own swap chain and renderer. The D3D11 device is shared across all surfaces.

## Device creation

The engine creates a D3D11 device with `D3D_DRIVER_TYPE_HARDWARE` first. If that fails (headless VMs, CI runners, Remote Desktop), it falls back to `D3D_DRIVER_TYPE_WARP` (software rasterizer). The device is created with `D3D11_CREATE_DEVICE_BGRA_SUPPORT` for Direct2D interop.

## Swap chain

Each surface's swap chain uses:
- `DXGI_FORMAT_B8G8R8A8_UNORM` (D2D native format)
- `DXGI_ALPHA_MODE_PREMULTIPLIED` (per-pixel transparency)
- `DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL` (required for composition)
- 2 buffers
- Composition mode (`CreateSwapChainForComposition`)

The swap chain is bound to the HWND through a DirectComposition visual tree. DWM composites the output onto the desktop.

## Window setup

All surface windows use `WS_EX_NOREDIRECTIONBITMAP`. This tells DWM not to allocate a redirection surface for the window. Instead, the swap chain's composition surface is used directly. Combined with `WS_EX_TOPMOST`, `WS_EX_TOOLWINDOW` (no taskbar entry), and `WS_EX_NOACTIVATE` (no focus stealing), this produces a transparent topmost window that does not appear in Alt-Tab.

## Render cycle

Each frame follows this sequence:

1. Check scene graph dirty flag via `take_dirty()`; skip if clean
2. `GetBuffer(0)` to get the back buffer DXGI surface
3. `CreateBitmapFromDxgiSurface` to create a D2D bitmap target
4. `BeginDraw` on the D2D device context
5. `Clear` with fully transparent color
6. Render elements back-to-front in insertion order:
   - **Rect**: `FillRoundedRectangle` or `FillRectangle`, optional border via `DrawRoundedRectangle`
   - **Text**: `DrawText` with DirectWrite text format (font family, size, bold, italic)
   - **Image**: `DrawBitmap` from cached D2D bitmap (created from premultiplied RGBA data)
7. `EndDraw`
8. `Present(1, 0)` (vsync'd)
9. `IDCompositionDevice::Commit` to push the new frame to DWM

## Scene graph

Each surface maintains a `SceneGraph` backed by `IndexMap<String, Element>`. The `IndexMap` preserves insertion order, which determines z-order: elements added later draw on top.

Elements are addressed by string keys. Setting an element with an existing key replaces it in place (preserving z-order). Removing an element shifts later elements forward.

A `dirty` flag is set whenever elements are added, updated, or removed. The renderer checks `take_dirty()` before each frame. If the scene is clean, no GPU work happens.

## DPI handling

The engine declares per-monitor DPI awareness v2 via the application manifest. Each surface tracks its current DPI scale factor. When `WM_DPICHANGED` arrives:

1. The window procedure queues a `DpiChangeEvent`
2. The engine resizes the swap chain to match the new DPI
3. The DPI scale is updated
4. The scene is marked dirty for re-render

All public API coordinates are in logical pixels. Anchor offsets are scaled by the target monitor's DPI factor.

## Custom draw

The `DrawOp` escape hatch lets consumers submit procedural drawing operations: `FillRect`, `FillRoundedRect`, `StrokeRect`, `DrawLine`, `FillEllipse`, `DrawText`. These are rendered after the scene graph on the same frame.

Custom draw is one-shot. The next scene graph change (any `set_*` or `remove` call) triggers a normal scene graph render that overwrites the custom draw content. If you need persistent custom draw, resubmit the ops after each scene change.

Custom draw is only available in-process (Rust and C APIs). It requires direct access to the D2D device context, which cannot cross process boundaries. The JSON-RPC host and Node.js addon do not expose it.

## GPU device loss recovery

The engine detects device loss during `Present` when DXGI returns `DXGI_ERROR_DEVICE_REMOVED` or `DXGI_ERROR_DEVICE_RESET`. Recovery:

1. Destroy all GPU resources (swap chains, renderers, D2D device)
2. Create a new D3D11 device (hardware, WARP fallback)
3. Rebuild the `SurfaceRenderer` for every active surface
4. Re-render all scene graphs (preserved in memory)
5. Emit a `DeviceRecovered` event to the consumer

The consumer does not need to take any action. All elements are restored automatically. Custom draw content is lost and must be resubmitted if needed.
