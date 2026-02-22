set up winpane-ffi crate with cbindgen and error handling

Transforms the winpane-ffi stub into a buildable cdylib that produces winpane.dll, with cbindgen header generation and thread-local error handling infrastructure (ffi_try macro, panic catching, null-pointer validation) ready for all subsequent FFI functions.
