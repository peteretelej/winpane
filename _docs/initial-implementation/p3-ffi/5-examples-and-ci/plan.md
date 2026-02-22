# Phase 5: Examples and CI - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p3-ffi/learnings.md`
2. `_docs/initial-implementation/p3-ffi/initial-plan.md` Phases 10-12
3. `_docs/initial-implementation/p3-ffi/5-examples-and-ci/spec.md`
4. `crates/winpane-ffi/include/winpane.h` (generated header to verify)
5. `.github/workflows/ci.yml` (current CI config)
6. `_docs/initial-implementation/phases-progress.md`

## Implementation Checklist

- [ ] Create `examples/c/hello_hud.c` with retained-mode API demo
- [ ] Create `examples/c/custom_draw.c` with canvas API demo
- [ ] Create `examples/c/CMakeLists.txt` for CMake build
- [ ] Create `examples/c/build.bat` for MSVC build
- [ ] Delete `examples/c/.gitkeep`
- [ ] Create `crates/winpane-ffi/include/winpane.def` with all 35 exported symbols
- [ ] Add C header verification step to `.github/workflows/ci.yml`
- [ ] Update `_docs/initial-implementation/phases-progress.md` with P3 completion notes
- [ ] Run `cargo fmt --all`
- [ ] Mark phase complete in root plan.md

## Implementation Summary

*(To be filled after implementation)*
