# Breakdown Review

## Agent Review Summary
- **Claude**: Found 4 critical, 4 important, and 5 minor issues. Strong on DLL naming, poll_event semantics, cbindgen version, CI gaps, canvas safety, config size validation, and event field naming.
- **Kimi**: Found 4 critical issues (overlapping with Claude) plus minor documentation gaps. Strong on library naming in build scripts, cbindgen version verification via web search.
- **Agreement areas**: cbindgen version outdated (0.27 -> 0.29), poll_event return semantics inconsistency, CI needs ilammy/msvc-dev-cmd, DLL/library naming mismatch
- **Unique finds**: Claude caught config `size` field not validated, canvas dangling pointer safety, event field `type` vs `event_type` C++ keyword clash, missing .gitignore, D2D_POINT_2F vagueness. Kimi caught build.bat lib name mismatch, UTF-8 key truncation risk.

## Requirements Coverage
- [x] All proposal requirements mapped to phases: yes, all 35 functions covered across 5 phases
- [x] No unjustified extra work: the `staticlib` crate-type addition is reasonable
- [x] Phase ordering is sound: 1 (core types) -> 2 (crate infra) -> 3 (FFI types) -> 4 (FFI functions) -> 5 (examples/CI)

## Phase Reviews

### Phase 1: Custom Draw Core
**Status**: Fixed
- D2D_POINT_2F guidance made specific: existing renderer doesn't use point/ellipse types, so can't "check existing code"; added concrete guidance to try `Direct2D::Common::D2D_POINT_2F` first, fall back to `windows_numerics::Vector2`.

### Phase 2: FFI Crate Setup
**Status**: Fixed
- cbindgen version updated from 0.27 to 0.29 (latest stable)
- Added `name = "winpane"` to `[lib]` in Cargo.toml so output is `winpane.dll`/`winpane.lib` (not `winpane_ffi.*`)

### Phase 3: FFI Types and Handles
**Status**: Fixed
- poll_event: changed to return 1 for no event (matching proposal), with implementation note that ffi_try! can't be used directly
- Config `to_rust()`: added size field validation (`size >= sizeof(Self)`)
- Event field: documented `event_type` name divergence from proposal draft (C++ keyword `type` avoidance)
- Key buffer: documented UTF-8 truncation risk for multi-byte sequences

### Phase 4: FFI Functions
**Status**: Fixed
- Canvas end_draw: added documentation requirement about dangling pointer invalidation

### Phase 5: Examples and CI
**Status**: Fixed
- CI: definitively added `ilammy/msvc-dev-cmd@v1` step (removed conditional language)
- C examples: added notes about `event_type` field name, poll_event return values, error string invalidation
- Library names: changed CMakeLists.txt and build.bat from `winpane_ffi` to `winpane`
- Added `.gitignore` entry for generated `winpane.h`

## Applied Fixes

1. **Phase 1 spec**: D2D_POINT_2F guidance made concrete (reference P0 gotchas, try Common::D2D_POINT_2F first)
2. **Phase 2 spec**: Cargo.toml added `name = "winpane"` under `[lib]`, cbindgen updated to 0.29
3. **Phase 3 spec**: poll_event returns 1 for no event (not 0), config size validation added, event_type naming note added, key buffer truncation documented
4. **Phase 4 spec**: Canvas dangling pointer documentation requirement added
5. **Phase 5 spec**: `ilammy/msvc-dev-cmd@v1` added definitively, library names fixed to `winpane`, .gitignore for header added, C example notes for event_type/poll_event/error string added
6. **initial-plan.md**: cbindgen 0.29, `name = "winpane"` in Cargo.toml, poll_event returns 1 for None, CI uses ilammy/msvc-dev-cmd, library names fixed in CMake/build.bat

## Open Questions

None. All issues were unambiguous and fixed directly.
