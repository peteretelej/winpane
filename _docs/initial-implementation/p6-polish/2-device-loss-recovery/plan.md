# Phase 2: Device Loss Recovery - Plan

## Required Reading

- `_docs/initial-implementation/p6-polish/learnings.md`
- `_docs/initial-implementation/p6-polish/2-device-loss-recovery/spec.md`
- `_docs/initial-implementation/p6-polish/initial-plan.md` (Steps 4-5)

## Implementation Checklist

- [x] Add `RenderError` enum to `crates/winpane-core/src/renderer.rs`
- [x] Modify `render()` to return `Result<(), RenderError>` and check `EndDraw`/`Present` for device loss
- [x] Modify `execute_draw_ops()` with the same device loss checks
- [x] Add `DeviceRecovered` variant to `Event` enum in `crates/winpane-core/src/types.rs`
- [x] Refactor `SurfaceRenderer::new()` to extract `create_device_resources()` method
- [x] Verify `GpuResources` variable is `let mut` in `engine_thread_main`
- [x] Add device loss recovery block after render pass in `crates/winpane-core/src/engine.rs`
- [x] Add `DeviceRecovered` match arm in FFI event conversion (`crates/winpane-ffi/src/lib.rs`)
- [x] Add `DeviceRecovered` match arm in host `event_to_json()` (`crates/winpane-host/src/dispatch.rs`)
- [x] Add `DeviceRecovered` match arm in Node.js `convert_event()` (`bindings/node/src/lib.rs`)
- [x] Run `cargo fmt --all` and verify `cargo fmt --all -- --check` passes
- [x] Mark phase complete

## Implementation Summary

Added GPU device loss detection and automatic recovery:

1. **RenderError enum** - New `pub(crate)` error type in renderer.rs with `DeviceLost` and `Other` variants, plus `is_device_lost()` helper checking DXGI_ERROR_DEVICE_REMOVED/RESET.

2. **Device loss detection** - `render()` and `execute_draw_ops()` now return `Result<(), RenderError>` instead of `Result<(), Error>`. EndDraw and Present results are checked for device loss HRESULT codes before converting to generic errors.

3. **Resource recreation** - Extracted `create_device_resources(&mut self, gpu: &GpuResources)` from `SurfaceRenderer::new()`. Recreates swap chain, DirectComposition chain, and D2D device context in-place (COM smart pointers auto-release old references).

4. **Recovery orchestration** - `recover_device()` in engine.rs logs the removal reason, creates a new `GpuResources`, calls `create_device_resources` on every surface, marks scenes dirty, and sends `Event::DeviceRecovered`. Both the render loop and CustomDraw command handler trigger recovery on device loss.

5. **Event propagation** - `DeviceRecovered` variant added to all API layers: FFI (`WinpaneEventType::DeviceRecovered = 8`), JSON-RPC host (`"device_recovered"`), and Node.js addon (`"device_recovered"`).
