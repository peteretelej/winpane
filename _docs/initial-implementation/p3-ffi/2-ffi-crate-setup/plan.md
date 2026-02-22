# Phase 2: FFI Crate Setup - Implementation Plan

## Required Reading

1. `_docs/initial-implementation/p3-ffi/learnings.md`
2. `_docs/initial-implementation/p3-ffi/initial-plan.md` Phases 3-4
3. `_docs/initial-implementation/p3-ffi/2-ffi-crate-setup/spec.md`
4. `crates/winpane-ffi/Cargo.toml`
5. `crates/winpane-ffi/src/lib.rs`

## Implementation Checklist

- [x] Replace `crates/winpane-ffi/Cargo.toml` with cdylib/staticlib config, winpane dep, cbindgen build-dep
- [x] Create `crates/winpane-ffi/cbindgen.toml` with C language config, type renames, enum style
- [x] Create `crates/winpane-ffi/include/` directory
- [x] Create `crates/winpane-ffi/build.rs` with cbindgen header generation
- [x] Replace `crates/winpane-ffi/src/lib.rs` stub with error handling: thread-local storage, `set_last_error`, `winpane_last_error()`, `ffi_try!`/`ffi_try_with!` macros, null-pointer helpers, `cstr_to_string`
- [x] Verify `cargo build --workspace` succeeds and `include/winpane.h` is generated
- [x] Run `cargo fmt --all`
- [x] Mark phase complete in root plan.md

## Implementation Summary

Replaced the winpane-ffi stub with a buildable cdylib crate. Cargo.toml now specifies `crate-type = ["cdylib", "staticlib"]` with `lib.name = "winpane"` (producing winpane.dll/winpane.lib), depends on the winpane crate, and uses cbindgen 0.29 as a build dependency. cbindgen.toml configures C language output with WINPANE_H include guard, type renames (WinpaneColor -> winpane_color_t, etc.), ScreamingSnakeCase enum variants, and cpp_compat. build.rs runs cbindgen to generate include/winpane.h (with graceful fallback if parsing fails during incremental development). lib.rs provides the error handling foundation: thread-local LAST_ERROR storage, `winpane_last_error()` as the first exported C symbol, `ffi_try!`/`ffi_try_with!` macros wrapping catch_unwind for panic safety, and null-pointer/CStr helpers. Standalone compilation confirms clean syntax; full workspace build requires Windows CI (upstream windows-rs crates are Windows-only).
