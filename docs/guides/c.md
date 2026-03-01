# C Guide

## Setup

Build the FFI DLL from source:

```sh
cargo build -p winpane-ffi --release
```

This produces `target/release/winpane_ffi.dll` and the header is at `crates/winpane-ffi/include/winpane.h`.

To use in your project:
1. Copy `winpane_ffi.dll` next to your executable (or add to PATH)
2. Copy `winpane.h` to your include path
3. Link against `winpane_ffi.dll.lib` (the import library, also in `target/release/`)

A CMakeLists.txt is provided in `examples/c/` as a reference.

## Hello world

```c
#include "winpane.h"
#include <windows.h>
#include <stdio.h>

int main(void) {
    WinpaneContext *ctx;
    if (winpane_create(&ctx) != 0) {
        printf("Error: %s\n", winpane_last_error());
        return 1;
    }

    winpane_hud_config_t cfg = {
        .version = WINPANE_CONFIG_VERSION,
        .size = sizeof(winpane_hud_config_t),
        .placement_type = 1, /* Monitor */
        .monitor_index = 0,
        .monitor_anchor = 0, /* TopLeft */
        .monitor_margin = 40,
        .width = 300, .height = 100,
    };
    WinpaneSurface *hud;
    if (winpane_hud_create(ctx, &cfg, &hud) != 0) {
        printf("Error: %s\n", winpane_last_error());
        winpane_destroy(ctx);
        return 1;
    }

    winpane_color_t white = { 255, 255, 255, 255 };
    winpane_text_element_t text = {
        .text = "Hello from winpane",
        .x = 16, .y = 16,
        .font_size = 18,
        .color = white,
    };
    winpane_surface_set_text(hud, "msg", &text);
    winpane_surface_show(hud);

    Sleep(INFINITE);

    winpane_surface_destroy(hud);
    winpane_destroy(ctx);
    return 0;
}
```

## Error handling

All functions that can fail return `int`:
- `0` = success
- `-1` = error (call `winpane_last_error()` for the message)
- `-2` = internal panic (should not happen in normal operation)

```c
if (winpane_surface_set_text(hud, "label", &elem) != 0) {
    fprintf(stderr, "winpane error: %s\n", winpane_last_error());
}
```

`winpane_last_error()` returns a `const char*` that is valid until the next winpane call on the same thread. Do not free it.

## Config struct versioning

All creation functions take a config struct with `version` and `size` fields. Always set both:

```c
winpane_hud_config_t cfg = {
    .version = WINPANE_CONFIG_VERSION,
    .size = sizeof(winpane_hud_config_t),
    // ...
};
```

This enables forward compatibility. If a future SDK version adds fields, old code with a smaller `size` still works.

## Elements

**Text:**

```c
winpane_text_element_t text = {
    .text = "CPU: 42%",
    .x = 16, .y = 50,
    .font_size = 14,
    .color = { 100, 220, 160, 255 },
    .font_family = "Consolas",  // NULL for system default
    .bold = 1,
    .interactive = 0,
};
winpane_surface_set_text(hud, "cpu", &text);
```

**Rect:**

```c
winpane_rect_element_t rect = {
    .x = 0, .y = 0,
    .width = 300, .height = 100,
    .fill = { 20, 20, 30, 200 },
    .corner_radius = 8.0,
    .has_border = 0,  // set to 1 and fill border_color to draw border
    .border_width = 0,
    .interactive = 0,
};
winpane_surface_set_rect(hud, "bg", &rect);
```

**Image:**

```c
winpane_image_element_t img = {
    .x = 10, .y = 10,
    .width = 32, .height = 32,
    .data = pixel_buffer,       // RGBA8 premultiplied, row-major
    .data_len = 32 * 32 * 4,
    .data_width = 32,
    .data_height = 32,
    .interactive = 0,
};
winpane_surface_set_image(hud, "icon", &img);
```

Remove with `winpane_surface_remove(hud, "key")`.

String parameters (`text`, `font_family`, `key`) are copied internally. You can free or reuse the buffer after the call returns.

## Interactive panels

```c
winpane_panel_config_t pcfg = {
    .version = WINPANE_CONFIG_VERSION,
    .size = sizeof(winpane_panel_config_t),
    .placement_type = 0, /* Position */
    .position_x = 200, .position_y = 200,
    .width = 260, .height = 100,
    .draggable = 1,
    .drag_height = 30,
};
WinpaneSurface *panel;
winpane_panel_create(ctx, &pcfg, &panel);

winpane_rect_element_t btn = {
    .x = 20, .y = 40, .width = 220, .height = 40,
    .fill = { 50, 80, 140, 200 },
    .corner_radius = 6,
    .interactive = 1,
};
winpane_surface_set_rect(panel, "btn", &btn);
winpane_surface_show(panel);

uint64_t panel_id = winpane_surface_id(panel);

winpane_event_t event;
while (1) {
    while (winpane_poll_event(ctx, &event) == 0) {
        if (event.event_type == WINPANE_EVENT_TYPE_ELEMENT_CLICKED
            && event.surface_id == panel_id
            && strcmp(event.key, "btn") == 0) {
            printf("Button clicked!\n");
        }
    }
    Sleep(16);
}
```

## Surface operations

```c
winpane_surface_show(surface);
winpane_surface_hide(surface);
winpane_surface_set_position(surface, 500, 300);
winpane_surface_set_size(surface, 400, 200);
winpane_surface_set_opacity(surface, 0.8f);
winpane_surface_set_backdrop(surface, WINPANE_BACKDROP_MICA);
winpane_surface_fade_in(surface, 300);
winpane_surface_fade_out(surface, 500);
```

## Custom draw

The canvas API provides in-process procedural drawing:

```c
WinpaneCanvas *canvas;
winpane_surface_begin_draw(surface, &canvas);

winpane_color_t blue = { 80, 160, 255, 255 };
winpane_canvas_fill_rect(canvas, 10, 10, 100, 50, blue);

winpane_color_t white = { 255, 255, 255, 255 };
winpane_canvas_draw_text(canvas, 20, 20, "Chart", 16.0, white);

winpane_surface_end_draw(surface);
```

The canvas pointer is invalid after `end_draw`. Custom draw is one-shot; the next scene graph change overwrites it.

## Cleanup

```c
winpane_surface_destroy(hud);    // destroy a surface
winpane_tray_destroy(tray);      // destroy a tray icon
winpane_destroy(ctx);            // shut down everything
```

Always destroy surfaces before the context. `winpane_destroy` cleans up remaining surfaces, but destroying them explicitly is cleaner.

## Next steps

- [Cookbook](../cookbook.md) - Recipes with Rust equivalents you can translate
- [FFI design](../design/ffi.md) - C ABI conventions, type mapping, error handling
- [Limitations](../limitations.md) - Known constraints
