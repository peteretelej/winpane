add FFI type definitions, opaque handles, and context/event functions

All repr(C) types that cross the FFI boundary are now defined: versioned config structs with version/size validation, element structs with C-to-Rust conversions, event enums, and a unified opaque surface handle that dispatches to Hud or Panel. Context lifecycle (create/destroy) and event polling are wired up.

- Config structs validate both version and minimum struct size for forward-compatibility
- Event polling returns 0/1/-1/-2 to distinguish event-available, no-event, error, and panic
- FfiSurface enum unifies Hud and Panel behind a single WinpaneSurface handle with 11 dispatch methods
