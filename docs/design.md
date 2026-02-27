# Design Overview

winpane creates companion UI surfaces on Windows using out-of-process DirectComposition windows. This document explains the design rationale and links to detailed implementation docs.

## The problem

Windows has no built-in API for "draw a transparent floating surface on top of other windows." Developers who need overlays, HUDs, or companion panels are left assembling Win32 window styles, swap chains, composition APIs, and input routing from scratch. The existing open-source options are narrow: hudhook injects into game processes (triggers anticheat), egui_overlay uses GLFW/OpenGL (no FFI story), screen_overlay uses GDI (no per-pixel transparency with capture exclusion).

winpane provides one SDK that handles all of this. You describe what to draw; the SDK manages windows, GPU rendering, DPI, input, and composition.

## Key decisions

**Out-of-process only.** winpane never injects DLLs or hooks render calls. Every surface is a standalone topmost Win32 window composited by DWM. This avoids anticheat detection, AV false positives, and stability issues from running inside another process's address space.

**DirectComposition, not legacy layered windows.** Windows provides two transparency paths: `WS_EX_LAYERED` with `UpdateLayeredWindow` (legacy, CPU-side alpha blending) and `WS_EX_NOREDIRECTIONBITMAP` with DirectComposition (GPU-native). The legacy path breaks `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` on Windows 11. winpane uses DirectComposition exclusively, which also avoids the GPU-to-CPU round-trip that legacy layered windows require.

**Retained-mode scene graph.** The API is not immediate-mode (no "draw this frame" callbacks). Instead, you set named elements (text, rects, images) with string keys. The SDK tracks what changed and only redraws when the scene is dirty. This works across FFI boundaries because state changes are simple key-value updates, not frame-synchronized draw calls.

**Internal SDK thread.** The engine runs on its own thread with its own Win32 message loop. Consumer threads send commands through an MPSC channel and poll events back. The consumer never creates windows, pumps messages, or touches GPU resources directly.

**Polled events, no callbacks.** Events (clicks, hovers, tray interactions) are delivered through a polled channel, not callbacks. This avoids reentrancy issues across FFI, keeps threading simple, and lets consumers integrate events into their own loop at their own pace.

## Surface types

| Type | Behavior | Use cases |
|------|----------|-----------|
| **Hud** | Click-through, topmost, no taskbar entry | Stats display, notifications, timers |
| **Panel** | Selective click-through, interactive elements, drag | Tool palettes, controls, floating UI |
| **Pip** | Live DWM thumbnail of another window | Preview panels, reference windows |
| **Tray** | System tray icon with popup and context menu | Background app controls, status |

All surface types share common operations: show/hide, position, size, opacity, anchoring, capture exclusion, backdrop effects, and fade animations. Hud and Panel additionally support the scene graph (text, rect, image elements). Pip has its own source region cropping. Tray has tooltip, icon, popup, and menu operations.

## Crate structure

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

`winpane-core` contains all Win32, DirectComposition, and Direct2D implementation. `winpane` wraps it in typed surface handles. External consumers (FFI, host, Node) depend only on `winpane`.

## Rendering stack

D3D11 device (hardware, WARP fallback) -> DXGI swap chain (composition, premultiplied alpha) -> Direct2D device context -> DirectComposition visual bound to HWND.

Text uses DirectWrite with grayscale antialiasing (ClearType is unavailable on transparent surfaces). Images are decoded to premultiplied RGBA and uploaded as D2D bitmaps.

See [rendering.md](design/rendering.md) for the full pipeline.

## Detailed design

- [Threading model](design/threading.md) - Engine thread, command queue, event delivery
- [Rendering pipeline](design/rendering.md) - GPU stack, scene graph, dirty tracking, custom draw
- [Surface types](design/surfaces.md) - Lifecycle, configuration, common operations
- [Input and hit testing](design/input.md) - Click-through, interactive elements, drag, hover
- [FFI design](design/ffi.md) - C ABI conventions, error handling, type mapping
- [Visual style](design/style.md) - Color palette, typography, spacing, component patterns

## Platform requirements

- Windows 10 version 1903+ (DirectComposition baseline, per-monitor DPI v2)
- Windows 10 version 2004+ for capture exclusion (`WDA_EXCLUDEFROMCAPTURE`)
- Windows 11 version 22H2+ for backdrop effects (Mica, Acrylic via `DwmSetWindowAttribute`)
