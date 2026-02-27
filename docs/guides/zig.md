# Zig Guide

Zig can consume winpane through the C ABI DLL using `@cImport` with the generated C header, or by loading the DLL at runtime with `std.DynLib`. Both approaches work; `@cImport` gives compile-time type checking while `std.DynLib` avoids needing the header at build time.

## Setup

Build the DLL and header:

```sh
cargo build -p winpane-ffi --release
```

Outputs:
- `target/release/winpane_ffi.dll` - the DLL
- `target/release/winpane_ffi.dll.lib` - the import library (for linking)
- `crates/winpane-ffi/include/winpane.h` - the C header

## Hello world with @cImport

Copy `winpane.h` to your project. In `build.zig`, add the include path and link the import library:

```zig
// build.zig
const exe = b.addExecutable(.{
    .name = "myapp",
    .root_source_file = b.path("src/main.zig"),
    .target = target,
    .optimize = optimize,
});
exe.addIncludePath(b.path("include")); // directory containing winpane.h
exe.addLibraryPath(b.path("lib"));     // directory containing winpane_ffi.dll.lib
exe.linkSystemLibrary("winpane_ffi");
b.installArtifact(exe);
```

Then in your source:

```zig
const std = @import("std");
const c = @cImport({
    @cInclude("winpane.h");
});

pub fn main() !void {
    // Create context
    var ctx: ?*c.WINPANE_WinpaneContext = null;
    if (c.winpane_create(&ctx) != 0) {
        const err = c.winpane_last_error();
        std.debug.print("create failed: {s}\n", .{std.mem.span(err)});
        return error.CreateFailed;
    }
    defer c.winpane_destroy(ctx);

    // Create HUD
    var cfg = c.WINPANE_winpane_hud_config_t{
        .version = c.WINPANE_WINPANE_CONFIG_VERSION,
        .size = @sizeOf(c.WINPANE_winpane_hud_config_t),
        .x = 100,
        .y = 100,
        .width = 300,
        .height = 100,
    };

    var hud: ?*c.WINPANE_WinpaneSurface = null;
    if (c.winpane_hud_create(ctx, &cfg, &hud) != 0) {
        std.debug.print("hud create failed: {s}\n", .{std.mem.span(c.winpane_last_error())});
        return error.HudCreateFailed;
    }
    defer c.winpane_surface_destroy(hud);

    // Background rect
    var rect = c.WINPANE_winpane_rect_element_t{
        .x = 0,
        .y = 0,
        .width = 300,
        .height = 100,
        .fill = .{ .r = 20, .g = 20, .b = 30, .a = 200 },
        .corner_radius = 8.0,
        .has_border = 0,
        .border_color = .{ .r = 0, .g = 0, .b = 0, .a = 0 },
        .border_width = 0,
        .interactive = 0,
    };
    _ = c.winpane_surface_set_rect(hud, "bg", &rect);

    // Text
    var text = c.WINPANE_winpane_text_element_t{
        .text = "Hello from Zig",
        .x = 16,
        .y = 16,
        .font_size = 18,
        .color = .{ .r = 255, .g = 255, .b = 255, .a = 255 },
        .font_family = null,
        .bold = 0,
        .italic = 0,
        .interactive = 0,
    };
    _ = c.winpane_surface_set_text(hud, "msg", &text);

    _ = c.winpane_surface_show(hud);

    std.debug.print("HUD visible. Press Ctrl+C to exit.\n", .{});
    while (true) {
        std.time.sleep(1_000_000_000); // 1 second
    }
}
```

## Polling events

```zig
var event: c.WINPANE_winpane_event_t = undefined;
while (c.winpane_poll_event(ctx, &event) == 0) {
    switch (event.event_type) {
        c.WINPANE_WINPANE_EVENT_TYPE_T_ELEMENT_CLICKED => {
            const key = std.mem.sliceTo(&event.key, 0);
            std.debug.print("clicked: surface={d} key={s}\n", .{ event.surface_id, key });
        },
        c.WINPANE_WINPANE_EVENT_TYPE_T_TRAY_MENU_ITEM_CLICKED => {
            if (event.menu_item_id == 99) {
                return; // quit
            }
        },
        else => {},
    }
}
```

## Interactive panel

```zig
var pcfg = c.WINPANE_winpane_panel_config_t{
    .version = c.WINPANE_WINPANE_CONFIG_VERSION,
    .size = @sizeOf(c.WINPANE_winpane_panel_config_t),
    .x = 200,
    .y = 200,
    .width = 260,
    .height = 100,
    .draggable = 1,
    .drag_height = 30,
};

var panel: ?*c.WINPANE_WinpaneSurface = null;
_ = c.winpane_panel_create(ctx, &pcfg, &panel);
defer c.winpane_surface_destroy(panel);

var btn = c.WINPANE_winpane_rect_element_t{
    .x = 20,
    .y = 40,
    .width = 220,
    .height = 40,
    .fill = .{ .r = 50, .g = 80, .b = 140, .a = 200 },
    .corner_radius = 6,
    .has_border = 0,
    .border_color = .{ .r = 0, .g = 0, .b = 0, .a = 0 },
    .border_width = 0,
    .interactive = 1,
};
_ = c.winpane_surface_set_rect(panel, "btn", &btn);
_ = c.winpane_surface_show(panel);

const panel_id = c.winpane_surface_id(panel);

while (true) {
    var event: c.WINPANE_winpane_event_t = undefined;
    while (c.winpane_poll_event(ctx, &event) == 0) {
        if (event.event_type == c.WINPANE_WINPANE_EVENT_TYPE_T_ELEMENT_CLICKED and
            event.surface_id == panel_id)
        {
            const key = std.mem.sliceTo(&event.key, 0);
            if (std.mem.eql(u8, key, "btn")) {
                std.debug.print("Button clicked!\n", .{});
            }
        }
    }
    std.time.sleep(16_000_000); // ~60fps
}
```

## Surface operations

```zig
_ = c.winpane_surface_show(surface);
_ = c.winpane_surface_hide(surface);
_ = c.winpane_surface_set_position(surface, 500, 300);
_ = c.winpane_surface_set_size(surface, 400, 200);
_ = c.winpane_surface_set_opacity(surface, 0.8);
_ = c.winpane_surface_set_backdrop(surface, c.WINPANE_WINPANE_BACKDROP_MICA);
_ = c.winpane_surface_fade_in(surface, 300);
_ = c.winpane_surface_fade_out(surface, 500);
_ = c.winpane_surface_set_capture_excluded(surface, 1);
_ = c.winpane_surface_anchor_to(surface, target_hwnd, c.WINPANE_WINPANE_ANCHOR_TOP_RIGHT, 8, 0);
_ = c.winpane_surface_unanchor(surface);
```

## Runtime DLL loading

If you prefer to load the DLL at runtime (avoids needing the import library at build time):

```zig
const std = @import("std");

pub fn main() !void {
    var lib = try std.DynLib.open("winpane_ffi.dll");
    defer lib.close();

    const create = lib.lookup(
        *const fn (**anyopaque) callconv(.C) i32,
        "winpane_create",
    ) orelse return error.SymbolNotFound;

    var ctx: *anyopaque = undefined;
    if (create(&ctx) != 0) {
        return error.CreateFailed;
    }
    // ... lookup and call other functions similarly
}
```

This is more verbose but useful for optional winpane integration where the DLL might not be present.

## Error handling pattern

Every winpane call that returns `i32` follows the same convention: `0` is success, `-1` is error, `-2` is panic. Wrap calls in a helper:

```zig
fn check(ret: i32) !void {
    if (ret == 0) return;
    const err = c.winpane_last_error();
    if (err != null) {
        std.debug.print("winpane error: {s}\n", .{std.mem.span(err)});
    }
    return error.WinpaneError;
}

// Usage
try check(c.winpane_surface_show(hud));
```

## Next steps

- [FFI design](../design/ffi.md) - C ABI conventions, type mapping, error handling
- [C guide](c.md) - Similar patterns, useful reference
- [Cookbook](../cookbook.md) - Recipes (Rust examples, translate the struct layouts)
- [Limitations](../limitations.md) - Known constraints
