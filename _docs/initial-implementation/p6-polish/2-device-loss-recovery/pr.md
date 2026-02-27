# Phase 2: Proposed Commit

add gpu device loss detection and automatic recovery

Detect DXGI_ERROR_DEVICE_REMOVED/RESET from EndDraw and Present calls, then recreate all GPU and DirectComposition resources from the retained scene graph. A DeviceRecovered event notifies consumers after successful recovery.

- Extracted `create_device_resources()` from `SurfaceRenderer::new()` for in-place resource replacement
- Propagated DeviceRecovered event through FFI, JSON-RPC host, and Node.js addon
