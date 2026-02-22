# P3: C ABI & FFI - Implementation Plan

Wraps the winpane Rust API in a C ABI (`winpane-ffi` crate producing `winpane.dll`), making winpane consumable from C, C++, Go, Zig, C#, Python, and any language with C FFI support. Also adds a custom draw escape hatch (DrawOp pipeline) for rendering beyond the declarative retained-mode primitives.

Reference: `proposal.md` for architecture decisions, `initial-plan.md` for full implementation details.

## Phases

- [x] Phase 1: [Custom Draw Core](1-custom-draw-core/spec.md) - DrawOp type, Command variant, renderer execution, public Rust API, Rust example
- [x] Phase 2: [FFI Crate Setup](2-ffi-crate-setup/spec.md) - Cargo.toml, cbindgen, build.rs, error handling infra, helper utilities
- [x] Phase 3: [FFI Types and Handles](3-ffi-types-and-handles/spec.md) - repr(C) types, conversions, opaque handles, context lifecycle, event polling
- [ ] Phase 4: [FFI Functions](4-ffi-functions/spec.md) - Surface creation/ops, tray functions, canvas functions
- [ ] Phase 5: [Examples and CI](5-examples-and-ci/spec.md) - C examples, CMake/build.bat, CI header check, .def file, progress update
