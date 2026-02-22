add surface, tray, and canvas FFI functions

Implements the remaining 31 extern "C" functions that complete the FFI layer: surface creation and operations, tray management, and the canvas custom draw pipeline. The crate now exports 35 functions total, covering the full winpane API surface for C consumers.
