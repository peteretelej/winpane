# P3: C ABI & FFI - Review Findings (Kimi)

## Web Search Result: cbindgen Version

**Finding:** The spec uses `cbindgen = "0.27"` but the latest stable version is **0.29.2**.

Per crates.io and the web search results:
- Latest: 0.29.2
- Previous: 0.28.0, 0.27.0 (released August 2024), 0.26.0 (September 2023)

**Recommendation:** Update `crates/winpane-ffi/Cargo.toml` to use `cbindgen = "0.29"` to stay current. Version 0.27 is not the latest stable as claimed in `initial-plan.md` line 630.

---

## Requirements Coverage

### Proposal Requirements (All Covered)

| Requirement | Status | Location in Plan |
|-------------|--------|------------------|
| Opaque pointer handles | Covered | Phase 3 spec, section 7 |
| Unified surface handle (FfiSurface enum) | Covered | Phase 3 spec, section 7.1 |
| Error handling (0/-1/-2 codes, thread-local) | Covered | Phase 2 spec, section 4 |
| Struct versioning (version + size fields) | Covered | Phase 3 spec, section 3 |
| Custom draw pipeline (DrawOp, canvas) | Covered | Phase 1 spec + Phase 4 spec section 4 |
| cbindgen header generation | Covered | Phase 2 spec, sections 2-3 |
| 35 total functions | Covered | Proposal section "Functions (35 total)" |

### Phase Ordering Assessment

The phase ordering is **sound**:
1. Phase 1: Custom draw core (needed for FFI canvas)
2. Phase 2: FFI crate setup (foundation for all FFI work)
3. Phase 3: Types and handles (prerequisite for functions)
4. Phase 4: FFI functions (uses types from Phase 3)
5. Phase 5: Examples and CI (validates everything works)

**No unjustified extra work** - everything ties back to the proposal requirements.

---

## Phase-by-Phase Findings

### Phase 1: Custom Draw Core

**Issue 1: File line references are hypothetical**
- `spec.md` references specific line numbers (line 118, 67, 475, 300, 169, 258)
- These references assume the files already exist with specific content
- Since source files aren't in this directory, implementer will need to verify actual line numbers during implementation
- **Fix:** Add note: "Line numbers are approximate - verify against current source"

**Issue 2: Missing D2D_POINT_2F detail**
- `spec.md` notes to check existing code for `D2D_POINT_2F` vs `Vector2`
- Good that this is noted as a potential gotcha
- `initial-plan.md` has a more detailed mitigation (lines 2050-2052)

**Issue 3: DrawOp variant ordering inconsistency**
- `spec.md` says DrawOp goes "after ImageElement (line 118)" in types.rs
- `initial-plan.md` says "after MenuItem (after line 187)"
- **Fix:** Use `initial-plan.md`'s version - it matches the actual file structure better (ImageElement at 107, MenuItem at 183 per the source summary in `.agi-state/prompt.md`)

**Issue 4: DrawOp::Clear variant**
- `spec.md` lists `Clear(Color)` variant in DrawOp
- `proposal.md` C API has `winpane_canvas_clear(c, color)` function
- But `initial-plan.md` phase 1.1 doesn't include Clear in the DrawOp enum
- **Fix:** Add `Clear(Color)` to DrawOp in `initial-plan.md` phase 1.1 (it's in spec.md but missing from initial-plan)

### Phase 2: FFI Crate Setup

**Issue 5: cbindgen version outdated (Critical)**
- `spec.md` and `initial-plan.md` use `cbindgen = "0.27"`
- Latest stable is 0.29.2 (per web search)
- **Fix:** Change to `cbindgen = "0.29"` in both files

**Issue 6: No libc dependency mentioned**
- `spec.md` says: "No `libc` dependency needed; use `std::os::raw`"
- But this contradicts common FFI practice - need to verify this works with cbindgen
- **Note:** This seems intentional (using std types instead of libc), monitor during impl

**Issue 7: Missing crate-type warning**
- `spec.md` mentions `crate-type = ["cdylib", "staticlib"]`
- On Windows, `staticlib` may have linking issues with C runtime
- **Fix:** Add note: "Test static linking on Windows - may need /MT vs /MD flags"

### Phase 3: FFI Types and Handles

**Issue 8: poll_event semantics INCONSISTENCY (Critical)**
- `spec.md` section 9 says: "If Some(e): returns 0; If None: returns 0"
- `proposal.md` says: "returns 0 if event available, returns 1 if no event"
- Proposal C example shows: `if (winpane_poll_event(ctx, &event) == 0) { handle event }`
- **Fix:** `spec.md` is WRONG. Per proposal, it should be:
  - Returns 0 and fills event struct if event available
  - Returns 1 if no event (WINPANE_EVENT_NONE)
- Update Phase 3 spec section 9 and the poll_event function signature documentation

**Issue 9: WinpaneEvent key buffer size**
- Key buffer is `[u8; 256]` - allows 255 chars + null
- No UTF-8 multi-byte handling mentioned
- **Fix:** Document that key truncation may split multi-byte UTF-8 sequences

**Issue 10: FfiSurface enum dispatch methods**
- `spec.md` lists 11 dispatch methods on FfiSurface
- `initial-plan.md` phase 6.1 shows all 11 implemented correctly
- All good here - methods match the surface operations needed

### Phase 4: FFI Functions

**Issue 11: winpane_tray_set_popup complexity note**
- `spec.md` correctly notes: "Extracts Panel from FfiSurface enum, errors if Hud"
- This is a sharp edge in the unified handle design
- **Fix:** Document clearly in C header: "Passing a HUD surface returns error"

**Issue 12: winpane_surface_id returns 0 for null**
- Returns 0 if surface is null
- SurfaceId is u64, so 0 is technically a valid ID (though unlikely)
- **Fix:** Consider returning u64::MAX instead, or document that 0 means "invalid/null"

**Issue 13: Canvas functions count mismatch**
- `spec.md` says "12 canvas functions" in intro
- Lists: begin_draw, end_draw, clear, fill_rect, stroke_rect, draw_text, draw_line, fill_ellipse, stroke_ellipse, draw_image, fill_rounded_rect, stroke_rounded_rect
- That's actually 12 (2 lifecycle + 10 drawing), matches proposal
- Count is correct, my miscount

**Issue 14: winpane_canvas_draw_text missing font parameter**
- DrawOp::DrawText has font_size but no font_family parameter
- C API `winpane_canvas_draw_text` matches (no font family)
- Element structs (WinpaneTextElement) have font_family
- This is intentional per "Decisions from brainstorm" - thin wrapper doesn't expose full text layout
- **Fix:** Document this limitation: "Canvas text uses default system font only"

### Phase 5: Examples and CI

**Issue 15: MSVC dev cmd action needed**
- `spec.md` says CI needs MSVC on PATH for `cl /c /W4`
- Notes current ci.yml doesn't use `ilammy/msvc-dev-cmd`
- This is a real issue - will need the action for header verification
- **Fix:** Add to Phase 5 spec: "Add ilammy/msvc-dev-cmd@v1 before header verification step"

**Issue 16: CMakeLists.txt library name**
- Links against `winpane_ffi` but DLL is named `winpane`
- On Windows, import lib should be `winpane.lib` not `winpane_ffi.lib`
- **Fix:** Change `target_link_libraries(hello_hud winpane_ffi)` to `winpane` (or verify actual output name)

**Issue 17: build.bat library name**
- Links `winpane_ffi.lib` - same issue as CMake
- Should be `winpane.lib`

**Issue 18: .def file exports count**
- Lists 35 symbols but count shows 34 (missing `winpane_poll_event`?)
- Let me recount: last item is `winpane_canvas_stroke_rounded_rect`
- Actually includes `winpane_poll_event` (line 20 in Phase 5 spec exports list)
- Count is 35 - OK

---

## Cross-Cutting Concerns

### Source Code Accuracy

Since the source files (`crates/winpane-core/src/types.rs`, etc.) don't exist in the `_docs/initial-implementation/p3-ffi` directory (they're references to the main project), the line numbers and specific locations in the specs are **predictions** based on the structure described in `.agi-state/prompt.md`. 

**Risk:** When implementing, actual source locations may differ.
**Mitigation:** The specs have enough context (e.g., "after ImageElement", "after DestroyTray") to find the right location even if line numbers are off.

### API Completeness

All 35 functions from proposal are covered:
- 1 error function
- 2 context lifecycle
- 2 surface creation
- 11 surface operations  
- 6 tray functions
- 1 event polling
- 2 draw lifecycle (begin/end)
- 10 canvas drawing

Total: 35 - matches proposal exactly.

### Error Handling Consistency

- All functions use `ffi_try!` or `ffi_try_with!`
- Return codes: 0=success, -1=error (call winpane_last_error), -2=panic
- Thread-local error storage is sound
- Null pointer validation on all input pointers

### Safety Documentation

- `spec.md` has `#![allow(clippy::missing_safety_doc)]` in lib.rs
- FFI functions need safety docs even if suppressed for clippy
- **Fix:** Add safety documentation comments in the actual implementation

---

## Summary of Critical Issues

1. **cbindgen version outdated** (Phase 2) - Use 0.29 instead of 0.27
2. **poll_event return semantics** (Phase 3) - Fix spec.md to match proposal (return 1 for no event, not 0)
3. **DrawOp::Clear missing from initial-plan** (Phase 1) - Add Clear variant to DrawOp enum
4. **Library naming inconsistency** (Phase 5) - CMake/build.bat use wrong lib name

## Overall Assessment

The plan breakdown is **comprehensive and well-structured**. The phases build logically on each other. Most issues are minor (documentation, version numbers) rather than architectural. The unified surface handle design is clever and well-executed. The error handling pattern is sound.

**Confidence:** High - this plan will produce a working FFI layer if implemented as specified, with the 4 critical fixes above applied.

---

*Review completed: All specs and proposal analyzed, web search for cbindgen conducted, source references cross-checked where possible.*

---

## Orchestrator Feedback

- [incorporated] "cbindgen version outdated" - confirmed via web search, updated to 0.29 in specs
- [incorporated] "poll_event return semantics inconsistency" - confirmed, fixed in Phase 3 spec and initial-plan.md
- [incorporated] "MSVC dev cmd action needed" - confirmed, added definitively to Phase 5 spec
- [incorporated] "CMakeLists.txt library name" - resolved by adding `name = "winpane"` to Cargo.toml so output is `winpane.dll`; CMakeLists.txt needs `winpane` not `winpane_ffi`
- [disregarded] "DrawOp::Clear missing from initial-plan" - it IS present in initial-plan.md Phase 1.1 line 25; the variant was always there
- [disregarded] "File line references are hypothetical" - verified all line references against actual source files; they are accurate
- [disregarded] "DrawOp placement after ImageElement vs MenuItem" - spec says after ImageElement (line 118), which is the better location; placing before HudConfig keeps element types together
- [disregarded] "winpane_surface_id returns 0 for null" - SurfaceId counter starts at a runtime value; 0 is technically possible but extremely unlikely and this is standard C pattern
- [disregarded] "Safety docs suppressed" - the clippy suppression is intentional for FFI crates; safety is documented in the C header instead
- [incorporated] "Canvas text uses default font" - this is by design per proposal brainstorm decisions; documented in proposal out-of-scope section
- [disregarded] "No libc dependency" - using `std::os::raw` types is correct and works fine with cbindgen; no libc needed
- [disregarded] "staticlib C runtime flags" - runtime flag choice is a consumer concern, not SDK configuration
