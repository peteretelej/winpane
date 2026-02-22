# P3: C ABI & FFI - Breakdown Review (Claude)

## Requirements Coverage

### Gaps

1. **poll_event return value semantics are contradictory.** The proposal (section 6) says: "returns 0 and fills the event struct if an event is available, returns 1 if no event (WINPANE_EVENT_NONE)." The C example in the proposal uses `if (winpane_poll_event(ctx, &event) == 0)` - which only processes events when the return is 0, implying 1 = no event. But Phase 3 spec (section 9) says: "If Some(e): returns 0; If None: returns 0" - both paths return 0. These are mutually exclusive designs. **Fix:** Phase 3 spec should match the proposal: return 0 when an event is available, return 1 when no event. This matches the C example and gives the consumer a fast check without inspecting the event struct. Alternatively, if always-0 is preferred, the proposal and all C examples must be updated to check `event.event_type != WINPANE_EVENT_NONE` instead of checking the return value.

2. **DLL output name mismatch.** The proposal says the output is `winpane.dll + winpane.lib`. But crate name is `winpane-ffi`, so Cargo produces `winpane_ffi.dll + winpane_ffi.lib` (hyphens become underscores). Phase 2 spec repeats the incorrect claim: "cdylib produces winpane.dll + winpane.lib." Phase 5 .def file says `LIBRARY winpane` which won't match the actual DLL name. Phase 5 CMakeLists.txt correctly links `winpane_ffi`, contradicting both the proposal and the .def file. **Fix:** Phase 2 spec should add `[lib] name = "winpane"` under `[lib]` in Cargo.toml to force the output name, OR all references to `winpane.dll`/`winpane.lib` should be updated to `winpane_ffi.dll`/`winpane_ffi.lib` and the .def file updated to `LIBRARY winpane_ffi`. Adding `[lib] name = "winpane"` is cleaner.

3. **cbindgen version is outdated.** Phase 2 spec specifies `cbindgen = "0.27"`. The latest stable is **0.29.2** (released Oct 2025). Version 0.27 was released Aug 2024. **Fix:** Phase 2 spec should use `cbindgen = "0.29"`.

4. **Proposal lists `libc` as a dependency, specs don't.** Proposal "Dependencies" section says: "`libc` - C-compatible types (`c_char`, `c_int`)." Phase 2 spec correctly says: "No `libc` dependency needed; use `std::os::raw::{c_char, c_int}` from std." The specs are correct; the proposal has a stale reference. Not a blocking issue since specs override, but worth noting for consistency.

5. **Proposal file manifest has wrong filename.** Proposal plan section 1 and file manifest reference `crates/winpane-core/src/render.rs`. The actual file is `renderer.rs`. Phase specs all correctly reference `renderer.rs`. Minor proposal error, no impact on implementation.

### Extras (justified)

- Phase 2 adds `staticlib` to crate-type alongside `cdylib`. Not in the proposal, but a reasonable addition for consumers who want static linking. No objection.
- Phase 3 uses `event_type` instead of proposal's `type` for the WinpaneEvent field. This is correct - `type` is a C++ reserved keyword and would break `cpp_compat = true`. The proposal C header and usage example need updating to match, but the spec is right.

### Ordering

Phase ordering is sound. Each phase has clear dependencies on the previous one:
1. Custom draw core (Rust types) -> 2. FFI crate infra -> 3. FFI types/handles -> 4. FFI functions -> 5. Examples/CI

No issues here.

---

## Phase-by-phase Findings

### Phase 1: Custom Draw Core

1. **D2D_POINT_2F import is hand-wavy.** The spec says "Check existing code for the correct point type import... Use whatever the existing render_text method uses." But `render_text` does NOT use any point type - it only uses `D2D_RECT_F`. The `DrawLine` op needs `D2D_POINT_2F` (or its windows-rs 0.62 equivalent). P0 gotchas explicitly note: "`D2D_POINT_2F` may need `Vector2` from `windows-numerics` in windows-rs 0.62." **Fix:** Phase 1 spec should say: "For `DrawLine`, check whether `D2D_POINT_2F` is available in `windows::Win32::Graphics::Direct2D::Common`. If not (windows-rs 0.62+), use `windows_numerics::Vector2` or construct the point struct inline. Reference the P0 gotchas note."

2. **execute_draw_ops renders scene graph + custom ops in one pass.** The spec says: "Render retained-mode scene graph (iterate elements, call existing render_rect/render_text/render_image), then execute each DrawOp." This means every custom draw call re-renders the full scene. This is correct per the proposal design, but the spec should note the performance implication: frequent custom_draw calls are expensive because the entire scene is re-rendered each time. This is fine for the intended use case (per-frame overlays) but worth documenting.

3. **Line references are accurate.** Verified against actual source:
   - ImageElement ends at line 118: correct
   - command.rs import at line 4: correct
   - DestroyTray at line 67: correct
   - renderer.rs import at line 11: correct (actual: `use crate::types::{Error, ImageElement, RectElement, TextElement}`)
   - set_opacity starts at line 475: correct
   - engine.rs DestroyTray at line 300: correct
   - Hud.id() at lines 167-169: correct
   - Panel.id() at lines 256-258: correct

### Phase 2: FFI Crate Setup

1. **cbindgen version.** As noted above, should be `"0.29"` not `"0.27"`.

2. **build.rs cbindgen resilience.** The spec suggests wrapping generate in `if let Ok(...)` during development, then making it strict before Phase 5. This is practical but should state the final form explicitly: "Before Phase 5 completion, this MUST use `.expect()` (not `if let Ok`). The lenient form is a temporary development aid only."

3. **cbindgen.toml `[export].prefix = "WINPANE_"`.** This adds `WINPANE_` prefix to all exported types not covered by explicit rename rules. Since all public types ARE explicitly renamed in `[export.rename]`, the prefix only affects types accidentally not listed. This is a safety net but could cause confusing names for any type missed from the rename list. **Suggestion:** Either remove `prefix = "WINPANE_"` (since all types are explicitly renamed) or add a comment explaining it's a catch-all safety net.

4. **Missing `[lib] name = "winpane"` as noted in Requirements Coverage.** The Cargo.toml in Phase 2 should include this.

### Phase 3: FFI Types and Handles

1. **poll_event semantics.** As noted in Requirements Coverage, the spec contradicts the proposal. Must be reconciled.

2. **Config struct `size` field is checked but not used.** The spec says `to_rust()` checks `version == WINPANE_CONFIG_VERSION` and returns Err on mismatch. But the `size` field is never validated or used for forward-compatibility logic. The proposal says: "SDK reads only the fields it knows about based on the version number; new fields go at the end" and "If size is smaller than expected, fields beyond the consumer's struct size get defaults." **Fix:** The `to_rust()` methods should at minimum validate `size >= std::mem::size_of::<WinpaneXxxConfig>()` for the current version to catch obviously wrong sizes. Forward-compat (reading fewer fields from old consumers) can be deferred to when version 2 is added, but the size validation should exist from day one.

3. **WinpaneEvent field `event_type` vs proposal's `type`.** Addressed above. The spec is correct, proposal examples need updating. Phase 5 C examples must use `event.event_type`, not `event.type`.

4. **FfiSurface dispatch methods include `custom_draw()`.** The spec says FfiSurface has dispatch methods including `custom_draw()`. This requires `Hud` and `Panel` to both have `custom_draw()`, which Phase 1 adds. Correct dependency chain.

### Phase 4: FFI Functions

1. **Canvas lifetime safety issue.** The design stores `CanvasAccumulator` inside `WinpaneSurface`, and `WinpaneCanvas` holds a raw pointer (`*mut Vec<DrawOp>`) into the accumulator. After `end_draw` calls `surface.canvas.take()`, the accumulator is dropped and the canvas's pointer becomes dangling. If the consumer calls any canvas function after `end_draw`, it's use-after-free. The spec says "The canvas handle becomes dangling after this call" but doesn't prevent misuse. **Fix options (pick one):**
   - **(A)** Add a validity flag to WinpaneCanvas (e.g., `valid: bool`). `end_draw` sets it to false. Canvas functions check it and return -1 if invalid. Minimal overhead.
   - **(B)** Make canvas functions go through the surface: `winpane_canvas_fill_rect(surface, ...)` instead of `winpane_canvas_fill_rect(canvas, ...)`. This eliminates the dangling pointer entirely but changes the API.
   - **(C)** Document it as "undefined behavior to use canvas after end_draw" and accept the C-API norm. The simplest option but the most error-prone for consumers.
   - **Recommended: (A).** Adding a `valid` flag is 1 line per canvas function and prevents crashes.

2. **tray_set_popup correctly handles the Hud case.** The spec says it extracts Panel from FfiSurface and errors if it's a Hud. The Rust API `Tray::set_popup(&Panel)` only accepts Panel, so the FFI must enforce this. Correctly specified.

3. **Function count is correct.** Phase 4 adds 31 functions (2 + 11 + 6 + 12). Combined with Phase 2 (1: winpane_last_error) and Phase 3 (3: create, destroy, poll_event), total is 35. Matches the proposal.

### Phase 5: Examples and CI

1. **CI header verification needs `ilammy/msvc-dev-cmd`.** The spec says to use `cl /c /W4 ...` with `shell: cmd`. On GitHub `windows-latest` runners, `cl.exe` is NOT on PATH by default. The spec says: "If cl is not on PATH, use the ilammy/msvc-dev-cmd@v1 action before this step. Check whether CI already uses this action." The current ci.yml does NOT use it. **Fix:** Phase 5 spec should definitively state: "Add `- uses: ilammy/msvc-dev-cmd@v1` before the header verification step. The current CI does not include this action." Remove the conditional language.

2. **C example uses `event.type` (proposal) vs `event.event_type` (spec).** Phase 5 C examples must use `event_type` to match the Phase 3 struct definition. The spec should explicitly note this divergence from the proposal usage example.

3. **CMakeLists.txt links `winpane_ffi` but .def says `LIBRARY winpane`.** Already noted in Requirements Coverage. Must be reconciled.

4. **Missing `include/winpane.h` from .gitignore.** The header is auto-generated by build.rs. It should be in `.gitignore` to avoid committing generated artifacts. The spec doesn't mention this. **Fix:** Add `crates/winpane-ffi/include/winpane.h` to `.gitignore`. Or alternatively, commit it as a distribution artifact (some projects do this for consumer convenience) but then CI should verify it's up-to-date.

5. **Proposal verification item 4 says "compiles without warnings on MSVC, Clang, MinGW."** Phase 5 only adds MSVC verification (`cl /c /W4`). Clang and MinGW verification are not included. Either the verification criteria should be relaxed to MSVC-only, or Phase 5 should add Clang/MinGW checks (which would require installing those toolchains in CI). **Recommendation:** MSVC-only is sufficient for initial release. Note the gap and plan Clang/MinGW for P6.

---

## Cross-cutting Concerns

1. **Proposal vs spec drift on C API field/function naming.** The proposal C header uses `type` for the event type field; the specs correctly use `event_type`. The proposal C usage example will not compile against the actual generated header. Any documentation or examples derived from the proposal need updating. **Action:** Either update the proposal to use `event_type`, or add a note in Phase 3 spec explicitly stating: "This diverges from the proposal draft header, which used `type`. The Rust field name `event_type` is used instead because `type` is a C++ reserved keyword."

2. **All-in-one lib.rs for the FFI crate.** Phases 2-4 put everything in `crates/winpane-ffi/src/lib.rs`. For 35 functions plus types, conversions, macros, and handle definitions, this file will be 800+ lines. The proposal (section 5) hints at an alternative: "or split into src/types.rs". The specs should either commit to single-file (acceptable for a pure-FFI layer) or define the split. **Recommendation:** Single file is fine for an FFI crate. The file is mostly boilerplate conversions and thin wrappers. No need to split unless it exceeds ~1200 lines.

3. **Error string memory management is under-documented.** `winpane_last_error()` returns a `*const c_char` from thread-local CString storage. The pointer is "valid until the next winpane call on the same thread." This means any winpane call (even `winpane_poll_event`) could invalidate the error string. The C examples should demonstrate defensive usage: copy the string or print it immediately. Phase 5 examples do call `winpane_last_error()` immediately after errors, which is correct. But neither the spec nor the examples explicitly warn about the invalidation rule. **Fix:** Add a comment in the hello_hud.c example: `/* Note: winpane_last_error() pointer is invalidated by the next winpane call */`

4. **No unit tests for the FFI crate.** The verification criteria mention `cargo test --workspace` but no FFI-specific tests are defined. Phase 2 adds macros and helpers that could have tests (e.g., `cstr_to_string` with null, valid, and invalid UTF-8 inputs). **Recommendation:** Add basic unit tests in Phase 2 for `set_last_error`/`winpane_last_error` roundtrip and `cstr_to_string` edge cases. These run on any platform (no Win32 dependency for the error handling code).

---

## Summary

**Overall assessment:** The breakdown is solid. Phase ordering, dependency chains, and scope are well-defined. The specs are detailed enough for implementation with specific line numbers and code snippets.

**Critical issues (must fix):**
1. poll_event return value semantics: spec contradicts proposal (Phase 3)
2. DLL output name: `winpane_ffi.dll` vs expected `winpane.dll` (Phase 2)
3. cbindgen version: `0.27` is outdated, use `0.29` (Phase 2)
4. CI header check needs `ilammy/msvc-dev-cmd` (Phase 5)

**Important issues (should fix):**
5. Canvas dangling pointer after end_draw - add validity flag (Phase 4)
6. Config struct `size` field never validated (Phase 3)
7. Event field `type` vs `event_type` - reconcile proposal and specs (Phase 3/5)
8. .def file `LIBRARY winpane` name must match actual DLL name (Phase 5)

**Minor issues (nice to fix):**
9. D2D_POINT_2F guidance is vague - reference P0 gotchas (Phase 1)
10. Missing .gitignore for generated header (Phase 5)
11. No FFI unit tests for error handling infra (Phase 2)
12. cbindgen.toml `[export].prefix` interacts oddly with explicit renames (Phase 2)
13. Proposal references `render.rs` instead of `renderer.rs` (informational)

---

## Orchestrator Feedback

- [incorporated] "poll_event return value semantics contradictory" - confirmed, fixed in Phase 3 spec and initial-plan.md
- [incorporated] "DLL output name mismatch" - confirmed, added `name = "winpane"` to Phase 2 spec Cargo.toml
- [incorporated] "cbindgen version outdated" - confirmed, updated to 0.29 in Phase 2 spec and initial-plan.md
- [incorporated] "CI header check needs ilammy/msvc-dev-cmd" - confirmed, fixed in Phase 5 spec and initial-plan.md
- [incorporated] "Canvas dangling pointer after end_draw" - documented clearly in Phase 4 spec (option C: document as C-API norm)
- [incorporated] "Config struct size field never validated" - added size validation to Phase 3 spec
- [incorporated] "Event field type vs event_type" - added reconciliation note in Phase 3 spec and C example notes in Phase 5 spec
- [incorporated] ".def file LIBRARY name" - resolved by adding `name = "winpane"` to Cargo.toml, .def is now correct
- [incorporated] "D2D_POINT_2F guidance is vague" - fixed in Phase 1 spec with concrete guidance
- [incorporated] "Missing .gitignore for generated header" - added to Phase 5 spec
- [disregarded] "No FFI unit tests for error handling" - reasonable suggestion but out of scope for this review; implementer can add if time permits
- [disregarded] "cbindgen.toml prefix interacts oddly" - the prefix is a safety net for unmapped types; it's fine as-is
- [disregarded] "Proposal references render.rs" - informational only, no spec change needed
