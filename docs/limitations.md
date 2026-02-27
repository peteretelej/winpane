# Known Limitations

## 1. No ClearType on transparent surfaces

DirectWrite uses grayscale antialiasing instead of ClearType subpixel rendering on surfaces with premultiplied alpha transparency. This is a Windows limitation: ClearType relies on knowing the background color, which is undefined for transparent pixels.

**Workaround:** Use opaque background rects behind text-heavy areas. Alternatively, increase font sizes where grayscale AA is less noticeable.

## 2. No fullscreen exclusive overlay

winpane surfaces work over borderless windowed and standard windowed applications. They do not appear over Direct3D exclusive fullscreen windows because DWM composition is bypassed in exclusive fullscreen mode.

**Workaround:** Target applications should use borderless windowed mode (which most modern games and applications support).

## 3. Composed-flip latency

DirectComposition introduces 1-3ms of composition latency compared to direct swap chain presentation. This is fine for tooling overlays, companion panels, HUDs, and widgets but not suitable for frame-perfect game overlay rendering that requires sub-millisecond timing.

**Workaround:** None. This is inherent to the DWM composition model. For frame-perfect game overlays, in-process rendering hooks are needed.

## 4. Capture exclusion version requirements

`SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` requires Windows 10 version 2004 (build 19041) or later. On older versions, `set_capture_excluded(true)` silently degrades (the surface remains visible in screenshots).

**Workaround:** Check the Windows version at runtime if you need to inform the user that capture exclusion is unavailable.

## 5. DWM thumbnail stale on minimize

PiP surfaces use DWM thumbnail APIs, which show the last rendered frame when the source window is minimized. The thumbnail does not update while the source is minimized.

**Workaround:** Listen for `PipSourceClosed` events and hide or overlay the PiP surface when the source is no longer active.

## 6. Custom draw not available over IPC

The `custom_draw` / `DrawOp` escape hatch requires in-process access to the Direct2D device context. It is not exposed through the JSON-RPC protocol or the Node.js addon because the GPU resources cannot cross process boundaries.

**Workaround:** Use the Rust or C API for custom draw. For IPC consumers, use the retained-mode scene graph (text, rect, image elements), which covers most UI needs.

## 7. Backdrop requires Windows 11 22H2+

DWM backdrop effects (Mica, Acrylic) require Windows 11 version 22H2 or later. `set_backdrop()` is a silent no-op on older versions. Use `backdrop_supported()` to check at runtime.

**Workaround:** Design surfaces to look complete without a backdrop effect. Use a semi-transparent background rect as a fallback.

## 8. Single-threaded rendering

All GPU rendering happens on the engine thread's message loop. A heavy custom draw operation (many draw ops or large image uploads) can block message processing, causing input lag or delayed event delivery.

**Workaround:** Keep custom draw operations lightweight. For complex procedural rendering, break work into smaller batches across multiple frames.

## 9. No async event delivery

Events are delivered through a polled `mpsc::Receiver`. The consumer must call `poll_event()` regularly in their own loop. There are no callback listeners, async streams, or push-based event delivery.

**Workaround:** Poll events on a timer (e.g., every 16ms for 60fps responsiveness) or integrate the event receiver into your application's existing event loop.
