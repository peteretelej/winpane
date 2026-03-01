/*
 * hello_hud.c - Minimal winpane HUD overlay example (C)
 *
 * Demonstrates the retained-mode API: create a context, create a HUD
 * surface, add text and rect elements, show it, and poll for events.
 *
 * Build (MSVC Developer Command Prompt):
 *   cl /W4 /I ..\..\crates\winpane-ffi\include hello_hud.c ^
 *      /link /LIBPATH:..\..\target\debug winpane.lib
 *
 * Run:
 *   hello_hud.exe
 */

#include "winpane.h"
#include <stdio.h>
#include <windows.h>

/* Helper: check return code and print last error on failure. */
static int check(int rc, const char *fn) {
    if (rc < 0) {
        /* NOTE: the pointer from winpane_last_error() is invalidated
         * by the next winpane call on the same thread. */
        const char *err = winpane_last_error();
        fprintf(stderr, "%s failed (%d): %s\n", fn, rc, err ? err : "unknown");
    }
    return rc;
}

int main(void) {
    WinpaneContext *ctx = NULL;
    WinpaneSurface *hud = NULL;

    /* 1. Create context */
    if (check(winpane_create(&ctx), "winpane_create") < 0)
        return 1;

    /* 2. Create HUD with versioned config */
    winpane_hud_config_t config = {0};
    config.version = WINPANE_CONFIG_VERSION;
    config.size = sizeof(config);
    config.placement_type = 1; /* Monitor */
    config.monitor_index = 0;
    config.monitor_anchor = 0; /* TopLeft */
    config.monitor_margin = 40;
    config.width = 320;
    config.height = 200;

    if (check(winpane_hud_create(ctx, &config, &hud), "winpane_hud_create") < 0) {
        winpane_destroy(ctx);
        return 1;
    }

    /* 3. Add a dark rounded background rect */
    winpane_rect_element_t bg = {0};
    bg.x = 0.0f;
    bg.y = 0.0f;
    bg.width = 320.0f;
    bg.height = 200.0f;
    bg.fill = (winpane_color_t){15, 15, 25, 220};
    bg.corner_radius = 8.0f;
    bg.has_border = 1;
    bg.border_color = (winpane_color_t){60, 60, 100, 180};
    bg.border_width = 1.0f;
    bg.interactive = 0;
    check(winpane_surface_set_rect(hud, "bg", &bg), "set_rect(bg)");

    /* 4. Add title text (white, large) */
    winpane_text_element_t title = {0};
    title.text = "System Monitor";
    title.x = 20.0f;
    title.y = 15.0f;
    title.font_size = 18.0f;
    title.color = (winpane_color_t){255, 255, 255, 255};
    title.font_family = NULL;
    title.bold = 1;
    title.italic = 0;
    title.interactive = 0;
    check(winpane_surface_set_text(hud, "title", &title), "set_text(title)");

    /* 5. Add value text (colored, smaller) */
    winpane_text_element_t value = {0};
    value.text = "CPU: 42%  |  RAM: 8.2 GB";
    value.x = 20.0f;
    value.y = 60.0f;
    value.font_size = 14.0f;
    value.color = (winpane_color_t){100, 220, 160, 255};
    value.font_family = NULL;
    value.bold = 0;
    value.italic = 0;
    value.interactive = 1;
    check(winpane_surface_set_text(hud, "value", &value), "set_text(value)");

    /* 6. Show the surface */
    check(winpane_surface_show(hud), "surface_show");

    printf("winpane hello_hud: overlay on monitor 0. Press Ctrl+C to exit.\n");

    /* 7. Event loop */
    for (;;) {
        winpane_event_t event = {0};
        int rc = winpane_poll_event(ctx, &event);
        if (rc == 0) {
            /* Event available */
            if (event.event_type == WINPANE_EVENT_TYPE_ELEMENT_CLICKED) {
                printf("Clicked element: %s\n", (const char *)event.key);
            } else if (event.event_type == WINPANE_EVENT_TYPE_ELEMENT_HOVERED) {
                printf("Hovered element: %s\n", (const char *)event.key);
            }
        } else if (rc < 0) {
            fprintf(stderr, "poll_event error: %s\n",
                    winpane_last_error() ? winpane_last_error() : "unknown");
            break;
        }
        Sleep(16);
    }

    /* 8. Cleanup */
    winpane_surface_destroy(hud);
    winpane_destroy(ctx);
    return 0;
}
