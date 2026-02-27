# Phase 6: Examples & CI - Plan

## Required Reading

- `_docs/initial-implementation/p6-polish/learnings.md`
- `_docs/initial-implementation/p6-polish/6-examples-ci/spec.md`
- `_docs/initial-implementation/p6-polish/initial-plan.md` (Steps 12-13)

## Implementation Checklist

- [x] Add `#[derive(Default)]` to `PanelConfig` and `RectElement` in `crates/winpane-core/src/types.rs` (or spell out all fields in examples)
- [x] Create `examples/rust/backdrop_demo.rs`
- [x] Create `examples/rust/fade_demo.rs`
- [x] Create `examples/node/backdrop_demo.js`
- [x] Register new Rust examples in `crates/winpane/Cargo.toml` (NOT winpane-core, since examples use `winpane::*`)
- [x] Review `.github/workflows/ci.yml` for any needed changes
- [x] Run `cargo fmt --all` and verify `cargo fmt --all -- --check` passes
- [x] Run `cargo fmt -- --check` in `bindings/node/`
- [x] Update `_docs/initial-implementation/phases-progress.md` with P6 completion notes
- [x] Mark phase complete

## Implementation Summary

- Added `Default` derive to `PanelConfig` and manual `Default` impl for `RectElement` (with `Color::TRANSPARENT` fill) to support `..Default::default()` in examples
- Created `backdrop_demo.rs`: two side-by-side panels showing Mica and Acrylic backdrops
- Created `fade_demo.rs`: HUD with fade-in/out animations and text updates
- Created `backdrop_demo.js`: Node.js example switching between Mica and Acrylic with fade-out
- Registered both Rust examples in `crates/winpane/Cargo.toml`
- Reviewed CI: no changes needed (existing `cargo build --workspace --all-targets` builds examples, fmt/clippy cover new code)
- Fixed pre-existing formatting issues in `bindings/node/src/lib.rs`
