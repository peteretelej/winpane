//! winpane-ffi: C ABI bindings for winpane.
//!
//! Produces winpane.dll (cdylib) with extern "C" functions consumable
//! from any language with C FFI support (C, C++, Go, Zig, C#, Python).

#![allow(clippy::missing_safety_doc)] // FFI functions document safety via C header

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;

// --- Thread-local error storage ---

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(msg: impl fmt::Display) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = CString::new(msg.to_string()).ok();
    });
}

/// Returns the last error message, or NULL if no error.
/// The returned pointer is valid until the next winpane call on the same thread.
#[no_mangle]
pub extern "C" fn winpane_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |s| s.as_ptr())
    })
}

// --- ffi_try! macro ---
//
// Wraps every extern "C" function body in catch_unwind + Result handling.
// Returns 0 on success, -1 on error (with last_error set), -2 on panic.

macro_rules! ffi_try {
    ($body:expr) => {{
        match std::panic::catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(Ok(())) => 0_i32,
            Ok(Err(e)) => {
                set_last_error(&e);
                -1_i32
            }
            Err(_) => {
                set_last_error("panic caught at FFI boundary");
                -2_i32
            }
        }
    }};
}

// Variant for functions that return a value through an out-pointer.
// The Ok branch yields the value; error paths use early return.
macro_rules! ffi_try_with {
    ($body:expr) => {{
        match std::panic::catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(Ok(val)) => val,
            Ok(Err(e)) => {
                set_last_error(&e);
                return -1_i32;
            }
            Err(_) => {
                set_last_error("panic caught at FFI boundary");
                return -2_i32;
            }
        }
    }};
}

// --- Null pointer validation helpers ---

fn require_non_null<T>(ptr: *const T, name: &str) -> Result<(), String> {
    if ptr.is_null() {
        Err(format!("{name} is null"))
    } else {
        Ok(())
    }
}

fn require_non_null_mut<T>(ptr: *mut T, name: &str) -> Result<(), String> {
    if ptr.is_null() {
        Err(format!("{name} is null"))
    } else {
        Ok(())
    }
}

// --- CStr helper ---

/// # Safety
/// `ptr` must point to a valid null-terminated C string if non-null.
unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("string pointer is null".into());
    }
    // Safety: caller guarantees valid null-terminated UTF-8
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|e| format!("invalid UTF-8: {e}"))
}
