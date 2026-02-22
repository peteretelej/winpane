# Phase 5: Examples and Verification - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p4-advanced-surfaces/learnings.md`
2. `_docs/initial-implementation/p4-advanced-surfaces/initial-plan.md` Steps 12-13
3. `_docs/initial-implementation/p4-advanced-surfaces/5-examples-and-verification/spec.md`
4. `crates/winpane/Cargo.toml` (check existing example registration pattern)
5. `examples/rust/custom_draw.rs` (reference for example structure)

## Implementation Checklist

- [x] Create `examples/rust/pip_viewer.rs` with PiP thumbnail demo
- [x] Create `examples/rust/anchored_companion.rs` with anchored panel demo
- [x] Create `examples/rust/capture_excluded.rs` with capture exclusion demo
- [x] Register all three examples in the appropriate `Cargo.toml` (follow existing pattern)
- [x] Add `windows` dev-dependency if needed for `FindWindowW` in examples
- [x] Run `cargo fmt --all`
- [x] Run `cargo check --workspace` (or `cargo build --workspace` on Windows) and verify it passes
- [x] Run `cargo clippy --workspace -- -D warnings` and verify it passes
- [x] Mark phase complete in root plan.md

## Implementation Summary

Created three Rust examples demonstrating P4 features:

- `pip_viewer.rs`: PiP thumbnail demo using FindWindowW + Context::create_pip, polls for PipSourceClosed events
- `anchored_companion.rs`: Panel anchored to another window's top-right corner, tracks movement and hide/show
- `capture_excluded.rs`: HUD with capture exclusion enabled via set_capture_excluded(true)

Registered all three as `[[example]]` entries in `crates/winpane/Cargo.toml`. Added `windows` as dev-dependency with `Win32_UI_WindowsAndMessaging` and `Win32_Foundation` features for FindWindowW usage.

Verification:
- `cargo fmt --all -- --check`: passes
- `cargo check --workspace`: fails due to pre-existing `windows-future v0.3.2` transitive dependency issue (same as Phase 1, documented in learnings.md)
- Header file (`winpane.h`): contains all P4 types, enums, and 6 new function declarations
- Def file: 41 total exports confirmed
