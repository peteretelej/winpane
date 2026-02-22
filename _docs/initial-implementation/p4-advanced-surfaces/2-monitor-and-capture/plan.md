# Phase 2: Monitor and Capture Exclusion - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p4-advanced-surfaces/learnings.md`
2. `_docs/initial-implementation/p4-advanced-surfaces/initial-plan.md` Steps 4-6
3. `_docs/initial-implementation/p4-advanced-surfaces/2-monitor-and-capture/spec.md`
4. `crates/winpane-core/src/window.rs` (for `PENDING_DPI_CHANGES` pattern and DPI utilities)
5. `crates/winpane-core/src/lib.rs` (for module registration)

## Implementation Checklist

- [x] Create `crates/winpane-core/src/monitor.rs` with module-level doc comment and imports
- [x] Add `MonitorEvent` enum and `PENDING_MONITOR_EVENTS` thread-local (following `PENDING_DPI_CHANGES` pattern)
- [x] Add `WatchReason` enum and `Watch` struct
- [x] Add `WindowMonitor` struct with `watched` HashMap and `hooks` Vec (cfg-gated)
- [x] Implement `WindowMonitor` methods: `new`, `is_empty`, `watch`, `unwatch_surface`, `unwatch`, `get_watches`, `watched_hwnds`
- [x] Implement Windows-only `register_hooks` and `unregister_hooks` (and no-op stubs for non-Windows)
- [x] Implement `monitor_event_callback` (Windows-only extern "system" fn)
- [x] Implement `Drop` for `WindowMonitor`
- [x] Register `pub(crate) mod monitor;` in `crates/winpane-core/src/lib.rs`
- [x] Add `WINDOWS_BUILD_NUMBER` OnceLock, `get_windows_build_number()`, and `rtl_get_version_build()` to `crates/winpane-core/src/window.rs`
- [x] Add `supports_exclude_from_capture()` to `window.rs`
- [x] Add `set_capture_excluded(hwnd, excluded)` to `window.rs`
- [x] Run `cargo fmt --all`
- [x] Run `cargo check --workspace` and verify it passes
- [x] Mark phase complete in root plan.md

## Implementation Summary

Created `monitor.rs` with the `WindowMonitor` module providing `SetWinEventHook` infrastructure for PiP source monitoring and window anchoring. The module includes `MonitorEvent` enum with a thread-local queue (following the `PENDING_DPI_CHANGES` pattern), `WatchReason`/`Watch` tracking types, and the `WindowMonitor` struct with lazy hook registration and automatic cleanup via `Drop`. Added capture exclusion utilities to `window.rs`: build number detection via `RtlGetVersion` (dynamically loaded from ntdll.dll), `supports_exclude_from_capture()` for Win10 2004+ detection, and `set_capture_excluded()` using `SetWindowDisplayAffinity` with appropriate fallback to `WDA_MONITOR` on older builds.

Build check: `cargo check --workspace` fails with the pre-existing `windows-future v0.3.2` transitive dependency issue (documented in Phase 1 learnings). All new code is `#[cfg(target_os = "windows")]` gated or uses platform-independent types and compiles cleanly as part of the workspace compilation up to the dependency boundary.
