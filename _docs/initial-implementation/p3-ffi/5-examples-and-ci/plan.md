# Phase 5: Examples and CI - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p3-ffi/learnings.md`
2. `_docs/initial-implementation/p3-ffi/initial-plan.md` Phases 10-12
3. `_docs/initial-implementation/p3-ffi/5-examples-and-ci/spec.md`
4. `crates/winpane-ffi/include/winpane.h` (generated header to verify)
5. `.github/workflows/ci.yml` (current CI config)
6. `_docs/initial-implementation/phases-progress.md`

## Implementation Checklist

- [x] Create `examples/c/hello_hud.c` with retained-mode API demo
- [x] Create `examples/c/custom_draw.c` with canvas API demo
- [x] Create `examples/c/CMakeLists.txt` for CMake build
- [x] Create `examples/c/build.bat` for MSVC build
- [x] Delete `examples/c/.gitkeep`
- [x] Create `crates/winpane-ffi/include/winpane.def` with all 35 exported symbols
- [x] Add C header verification step to `.github/workflows/ci.yml`
- [x] Update `_docs/initial-implementation/phases-progress.md` with P3 completion notes
- [x] Run `cargo fmt --all`
- [x] Mark phase complete in root plan.md

## Implementation Summary

Created two C example programs validating the FFI API end-to-end. `hello_hud.c` demonstrates the retained-mode API (context creation, versioned HUD config, rect/text elements, event polling loop, cleanup). `custom_draw.c` demonstrates the canvas API (begin_draw, fill_rounded_rect, draw_text, draw_line, stroke_ellipse, stroke_rect, end_draw) rendering a bar chart. Both examples include proper error checking via `winpane_last_error()` and compile instructions in header comments.

Added CMakeLists.txt and build.bat for building the C examples. Created winpane.def listing all 35 exported DLL symbols. Added CI header verification (ilammy/msvc-dev-cmd + cl.exe compilation check). Added generated winpane.h to .gitignore. Updated phases-progress.md marking P3 complete.
