/*
 * custom_draw.c - Canvas/custom draw API example (C)
 *
 * Demonstrates the immediate-mode canvas API: begin_draw to get a canvas
 * handle, draw shapes and text, then end_draw to flush to the surface.
 * Renders a bar chart similar to the Rust custom_draw example.
 *
 * Build (MSVC Developer Command Prompt):
 *   cl /W4 /I ..\..\crates\winpane-ffi\include custom_draw.c ^
 *      /link /LIBPATH:..\..\target\debug winpane.lib
 *
 * Run:
 *   custom_draw.exe
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
    WinpaneCanvas *canvas = NULL;

    /* 1. Create context */
    if (check(winpane_create(&ctx), "winpane_create") < 0)
        return 1;

    /* 2. Create HUD */
    winpane_hud_config_t config = {0};
    config.version = WINPANE_CONFIG_VERSION;
    config.size = sizeof(config);
    config.x = 200;
    config.y = 200;
    config.width = 400;
    config.height = 300;

    if (check(winpane_hud_create(ctx, &config, &hud), "winpane_hud_create") < 0) {
        winpane_destroy(ctx);
        return 1;
    }

    /* 3. Add retained-mode background rect */
    winpane_rect_element_t bg = {0};
    bg.x = 0.0f;
    bg.y = 0.0f;
    bg.width = 400.0f;
    bg.height = 300.0f;
    bg.fill = (winpane_color_t){15, 15, 25, 220};
    bg.corner_radius = 8.0f;
    bg.has_border = 1;
    bg.border_color = (winpane_color_t){60, 60, 100, 180};
    bg.border_width = 1.0f;
    bg.interactive = 0;
    check(winpane_surface_set_rect(hud, "bg", &bg), "set_rect(bg)");

    /* 4. Show surface, brief sleep for window to appear */
    check(winpane_surface_show(hud), "surface_show");
    Sleep(200);

    /* 5. Begin custom draw session */
    if (check(winpane_surface_begin_draw(hud, &canvas), "begin_draw") < 0) {
        winpane_surface_destroy(hud);
        winpane_destroy(ctx);
        return 1;
    }

    /* -- Title -- */
    check(winpane_canvas_draw_text(canvas, 20.0f, 15.0f,
          "Weekly Activity", 18.0f,
          (winpane_color_t){255, 255, 255, 255}), "draw_text(title)");

    /* -- Horizontal baseline -- */
    check(winpane_canvas_draw_line(canvas,
          40.0f, 240.0f, 370.0f, 240.0f,
          (winpane_color_t){80, 80, 120, 200}, 1.0f), "draw_line(baseline)");

    /* -- Bar chart -- */
    {
        const float bar_values[4] = {0.7f, 0.45f, 0.9f, 0.3f};
        const char *bar_labels[4] = {"Mon", "Tue", "Wed", "Thu"};
        const winpane_color_t bar_colors[4] = {
            {80, 160, 255, 255},
            {100, 220, 160, 255},
            {255, 180, 80, 255},
            {255, 100, 120, 255},
        };
        const float bar_width = 60.0f;
        const float bar_max_height = 170.0f;
        const float start_x = 55.0f;
        const float spacing = 80.0f;
        int i;

        for (i = 0; i < 4; i++) {
            float x = start_x + (float)i * spacing;
            float bar_height = bar_values[i] * bar_max_height;
            float y = 240.0f - bar_height;

            /* Filled rounded rect for the bar */
            check(winpane_canvas_fill_rounded_rect(canvas,
                  x, y, bar_width, bar_height, 4.0f,
                  bar_colors[i]), "fill_rounded_rect(bar)");

            /* Label below bar */
            check(winpane_canvas_draw_text(canvas,
                  x + 15.0f, 248.0f,
                  bar_labels[i], 12.0f,
                  (winpane_color_t){160, 160, 180, 255}), "draw_text(label)");

            /* Value above bar */
            {
                char value_str[16];
                snprintf(value_str, sizeof(value_str), "%d%%",
                         (int)(bar_values[i] * 100.0f));
                check(winpane_canvas_draw_text(canvas,
                      x + 12.0f, y - 20.0f,
                      value_str, 11.0f,
                      bar_colors[i]), "draw_text(value)");
            }
        }
    }

    /* -- Decorative elements -- */

    /* Stroke ellipse in top-right corner */
    check(winpane_canvas_stroke_ellipse(canvas,
          370.0f, 30.0f, 12.0f, 12.0f,
          (winpane_color_t){100, 180, 255, 120}, 1.5f), "stroke_ellipse");

    /* Stroke rect border around chart area */
    check(winpane_canvas_stroke_rect(canvas,
          30.0f, 45.0f, 350.0f, 220.0f,
          (winpane_color_t){40, 40, 70, 100}, 1.0f), "stroke_rect(border)");

    /* 6. End draw - flush ops to surface */
    /* NOTE: canvas handle is invalid after this call. */
    check(winpane_surface_end_draw(hud), "end_draw");
    canvas = NULL;

    printf("winpane custom_draw: bar chart overlay at (200, 200).\n");
    printf("Press Ctrl+C to exit.\n");

    /* 7. Sleep loop to keep the overlay visible */
    for (;;) {
        Sleep(1000);
    }

    /* 8. Cleanup (unreachable in this example, but shown for correctness) */
    winpane_surface_destroy(hud);
    winpane_destroy(ctx);
    return 0;
}
