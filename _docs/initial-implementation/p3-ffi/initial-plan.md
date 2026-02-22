# P3: C ABI & FFI - Implementation Plan

> **Source**: `proposal.md` in this directory.
> **Pre-push**: `cargo fmt --all -- --check` before every push.

---

## Phase 1: Custom draw support in winpane-core

Add the `DrawOp` type and `Command::CustomDraw` to the core crate, with engine-side execution on the D2D render target. This is foundational; the FFI canvas API depends on it.

### 1.1 Add DrawOp enum to types.rs

**File:** `crates/winpane-core/src/types.rs`

Add after the `MenuItem` struct (after line 187):

```rust
// --- DrawOp ---

/// Low-level drawing operations for the custom draw escape hatch.
/// Accumulated by the FFI canvas and sent as a batch to the engine.
#[derive(Debug, Clone)]
pub enum DrawOp {
    Clear(Color),
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    StrokeRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        stroke_width: f32,
    },
    DrawText {
        x: f32,
        y: f32,
        text: String,
        font_size: f32,
        color: Color,
    },
    DrawLine {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        stroke_width: f32,
    },
    FillEllipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        color: Color,
    },
    StrokeEllipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        color: Color,
        stroke_width: f32,
    },
    DrawImage {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        rgba: Vec<u8>,
        img_width: u32,
        img_height: u32,
    },
    FillRoundedRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: Color,
    },
    StrokeRoundedRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: Color,
        stroke_width: f32,
    },
}
```

### 1.2 Add Command::CustomDraw to command.rs

**File:** `crates/winpane-core/src/command.rs`

Add import of `DrawOp` at the top (line 4):
```rust
use crate::types::{DrawOp, Error, HudConfig, MenuItem, PanelConfig, SurfaceId, TrayConfig, TrayId};
```

Add new variant after `DestroyTray(TrayId)` (after line 67):
```rust
    // --- New P3 commands ---
    CustomDraw {
        surface: SurfaceId,
        ops: Vec<DrawOp>,
    },
```

### 1.3 Add execute_draw_ops to renderer.rs

**File:** `crates/winpane-core/src/renderer.rs`

Add import at the top alongside existing imports:
```rust
use crate::types::{Color, DrawOp, Error, ImageElement, RectElement, TextElement};
```
(Add `Color` and `DrawOp` to the existing use statement on line 11.)

Add a new public method on `SurfaceRenderer`, after `set_opacity` (after line 490). This method performs a full begin/end draw cycle with the provided ops:

```rust
    /// Execute a batch of custom draw operations.
    /// Performs a full BeginDraw/EndDraw/Present cycle, rendering the scene
    /// graph first (if any), then the custom ops on top.
    pub unsafe fn execute_draw_ops(
        &self,
        scene: &SceneGraph,
        gpu: &GpuResources,
        ops: &[DrawOp],
    ) -> Result<(), Error> {
        let scale = self.dpi_scale;
        let phys_w = self.width as f32 * scale;
        let phys_h = self.height as f32 * scale;

        // Release current target
        self.dc.SetTarget(None);

        // Get new back buffer reference
        let surface: IDXGISurface = self
            .swapchain
            .GetBuffer(0)
            .map_err(|e| Error::RenderError(format!("GetBuffer: {e}")))?;

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0 * scale,
            dpiY: 96.0 * scale,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            ..Default::default()
        };

        let bitmap = self
            .dc
            .CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))
            .map_err(|e| Error::RenderError(format!("CreateBitmapFromDxgiSurface: {e}")))?;
        self.dc.SetTarget(&bitmap);

        // Begin drawing
        self.dc.BeginDraw();
        self.dc.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        // Render retained-mode scene graph first (base layer)
        for (_key, element) in scene.iter() {
            match element {
                Element::Rect(elem) => self.render_rect(elem, scale)?,
                Element::Text(elem) => self.render_text(elem, gpu, scale, phys_w, phys_h)?,
                Element::Image(elem) => self.render_image(elem, scale)?,
            }
        }

        // Execute custom draw ops on top
        for op in ops {
            self.execute_single_draw_op(op, gpu, scale, phys_w, phys_h)?;
        }

        self.dc
            .EndDraw(None, None)
            .map_err(|e| Error::RenderError(format!("EndDraw: {e}")))?;

        self.swapchain
            .Present(1, DXGI_PRESENT(0))
            .ok()
            .map_err(|e| Error::RenderError(format!("Present: {e}")))?;

        self.dcomp_device
            .Commit()
            .map_err(|e| Error::RenderError(format!("DComposition commit: {e}")))?;

        Ok(())
    }

    /// Execute a single DrawOp against the active D2D context.
    /// Must be called between BeginDraw and EndDraw.
    unsafe fn execute_single_draw_op(
        &self,
        op: &DrawOp,
        gpu: &GpuResources,
        scale: f32,
        surface_width: f32,
        surface_height: f32,
    ) -> Result<(), Error> {
        match op {
            DrawOp::Clear(color) => {
                self.dc.Clear(Some(&color.to_d2d_premultiplied()));
            }
            DrawOp::FillRect { x, y, width, height, color } => {
                let rect = D2D_RECT_F {
                    left: x * scale,
                    top: y * scale,
                    right: (x + width) * scale,
                    bottom: (y + height) * scale,
                };
                let brush = self.dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                self.dc.FillRectangle(&rect, &brush);
            }
            DrawOp::StrokeRect { x, y, width, height, color, stroke_width } => {
                let rect = D2D_RECT_F {
                    left: x * scale,
                    top: y * scale,
                    right: (x + width) * scale,
                    bottom: (y + height) * scale,
                };
                let brush = self.dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                self.dc.DrawRectangle(&rect, &brush, stroke_width * scale, None);
            }
            DrawOp::DrawText { x, y, text, font_size, color } => {
                // Reuse the same text rendering approach as render_text
                let format = gpu.dwrite_factory
                    .CreateTextFormat(
                        w!("Segoe UI"),
                        None,
                        DWRITE_FONT_WEIGHT_REGULAR,
                        DWRITE_FONT_STYLE_NORMAL,
                        DWRITE_FONT_STRETCH_NORMAL,
                        font_size * scale,
                        w!("en-us"),
                    )
                    .map_err(|e| Error::RenderError(format!("CreateTextFormat: {e}")))?;

                let text_utf16: Vec<u16> = text.encode_utf16().collect();
                let brush = self.dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;

                let layout_rect = D2D_RECT_F {
                    left: x * scale,
                    top: y * scale,
                    right: surface_width,
                    bottom: surface_height,
                };

                self.dc.DrawText(
                    &text_utf16,
                    &format,
                    &layout_rect as *const D2D_RECT_F,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            DrawOp::DrawLine { x1, y1, x2, y2, color, stroke_width } => {
                let brush = self.dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let p0 = D2D_POINT_2F { x: x1 * scale, y: y1 * scale };
                let p1 = D2D_POINT_2F { x: x2 * scale, y: y2 * scale };
                self.dc.DrawLine(p0, p1, &brush, stroke_width * scale, None);
            }
            DrawOp::FillEllipse { cx, cy, rx, ry, color } => {
                let brush = self.dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let ellipse = D2D1_ELLIPSE {
                    point: D2D_POINT_2F { x: cx * scale, y: cy * scale },
                    radiusX: rx * scale,
                    radiusY: ry * scale,
                };
                self.dc.FillEllipse(&ellipse, &brush);
            }
            DrawOp::StrokeEllipse { cx, cy, rx, ry, color, stroke_width } => {
                let brush = self.dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let ellipse = D2D1_ELLIPSE {
                    point: D2D_POINT_2F { x: cx * scale, y: cy * scale },
                    radiusX: rx * scale,
                    radiusY: ry * scale,
                };
                self.dc.DrawEllipse(&ellipse, &brush, stroke_width * scale, None);
            }
            DrawOp::DrawImage { x, y, width, height, rgba, img_width, img_height } => {
                // Reuse same RGBA->BGRA + CreateBitmap approach as render_image
                let bgra_data = rgba_to_bgra(rgba);
                let bmp = self.dc.CreateBitmap(
                    D2D_SIZE_U { width: *img_width, height: *img_height },
                    Some(bgra_data.as_ptr() as *const c_void),
                    *img_width * 4,
                    &D2D1_BITMAP_PROPERTIES1 {
                        pixelFormat: D2D1_PIXEL_FORMAT {
                            format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                        },
                        dpiX: 96.0,
                        dpiY: 96.0,
                        bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
                        ..Default::default()
                    },
                ).map_err(|e| Error::RenderError(format!("CreateBitmap: {e}")))?;

                let dest = D2D_RECT_F {
                    left: x * scale,
                    top: y * scale,
                    right: (x + width) * scale,
                    bottom: (y + height) * scale,
                };
                self.dc.DrawBitmap(
                    &bmp,
                    Some(&dest as *const D2D_RECT_F),
                    1.0,
                    D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
                    None,
                    None,
                );
            }
            DrawOp::FillRoundedRect { x, y, width, height, radius, color } => {
                let brush = self.dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let rr = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: x * scale,
                        top: y * scale,
                        right: (x + width) * scale,
                        bottom: (y + height) * scale,
                    },
                    radiusX: radius * scale,
                    radiusY: radius * scale,
                };
                self.dc.FillRoundedRectangle(&rr, &brush);
            }
            DrawOp::StrokeRoundedRect { x, y, width, height, radius, color, stroke_width } => {
                let brush = self.dc
                    .CreateSolidColorBrush(&color.to_d2d_premultiplied(), None)
                    .map_err(|e| Error::RenderError(format!("brush: {e}")))?;
                let rr = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: x * scale,
                        top: y * scale,
                        right: (x + width) * scale,
                        bottom: (y + height) * scale,
                    },
                    radiusX: radius * scale,
                    radiusY: radius * scale,
                };
                self.dc.DrawRoundedRectangle(&rr, &brush, stroke_width * scale, None);
            }
        }
        Ok(())
    }
```

**Note on D2D_POINT_2F:** In windows-rs 0.62, `D2D_POINT_2F` may need to be imported from `D2D1_Common` or use `windows_numerics::Vector2` per the P0 gotcha. Verify against the existing `render_text` method which successfully uses D2D point types. If `D2D_POINT_2F` isn't available, use `Vector2 { X: ..., Y: ... }` from `windows_numerics`.

### 1.4 Handle CustomDraw in engine.rs

**File:** `crates/winpane-core/src/engine.rs`

In the command match block (inside the `while let Ok(cmd) = cmd_rx.try_recv()` loop, around line 166-305), add a new arm after the `Command::DestroyTray` arm (after line 303):

```rust
                Command::CustomDraw { surface, ops } => {
                    if let Some(s) = surfaces.get(&surface) {
                        let _ = s.renderer.execute_draw_ops(&s.scene, &gpu, &ops);
                    }
                }
```

**Important:** When `CustomDraw` is processed, it renders the scene graph + ops and presents immediately. The surface's dirty flag does NOT need to be set because `execute_draw_ops` handles its own present cycle. However, note that the next time the scene graph changes (via SetElement), the normal `render()` path will run and overwrite the custom draw content. This is the expected behavior: custom draw is per-frame, not persistent. Document this.

### 1.5 Re-export DrawOp from winpane-core lib.rs

**File:** `crates/winpane-core/src/lib.rs`

Update the re-export line (line 14) to include DrawOp. The `pub use types::*` on line 14 already re-exports everything in types, so `DrawOp` is automatically included. No change needed here.

Verify: `DrawOp` is in `types.rs` which is `pub mod types` (line 10), and `pub use types::*` (line 14) already re-exports all public items.

---

## Phase 2: Custom draw in winpane public API

### 2.1 Add custom_draw() to Hud and Panel

**File:** `crates/winpane/src/lib.rs`

Add `DrawOp` to the re-export list (line 5-8):
```rust
pub use winpane_core::{
    Color, DrawOp, Error, Event, HudConfig, ImageElement, MenuItem, MouseButton, PanelConfig,
    RectElement, SurfaceId, TextElement, TrayConfig, TrayId,
};
```

Add `custom_draw` method to `Hud` impl, after `id()` method (after line 169):
```rust
    /// Execute custom draw operations on this surface.
    /// Renders the scene graph first, then the provided ops on top.
    /// This is a one-shot operation; the next scene graph change
    /// will overwrite the custom draw content.
    pub fn custom_draw(&self, ops: Vec<DrawOp>) {
        self.send(Command::CustomDraw {
            surface: self.id,
            ops,
        });
    }
```

Add identical `custom_draw` method to `Panel` impl, after its `id()` method (after line 258):
```rust
    pub fn custom_draw(&self, ops: Vec<DrawOp>) {
        self.send(Command::CustomDraw {
            surface: self.id,
            ops,
        });
    }
```

### 2.2 Rust custom draw example

**File:** `examples/rust/custom_draw.rs` (create)

```rust
//! Demo: custom draw escape hatch
//!
//! Creates a HUD and uses DrawOp to render custom content beyond
//! the retained-mode scene graph: filled rects, text, lines, ellipses.
//!
//! Run on Windows: cargo run -p winpane --example custom_draw

use winpane::{Color, Context, DrawOp, HudConfig, RectElement};

#[allow(clippy::print_stdout)]
fn main() -> Result<(), winpane::Error> {
    let ctx = Context::new()?;

    let hud = ctx.create_hud(HudConfig {
        x: 200,
        y: 200,
        width: 400,
        height: 300,
    })?;

    // Add a retained-mode background
    hud.set_rect(
        "bg",
        RectElement {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
            fill: Color::rgba(15, 15, 25, 220),
            corner_radius: 8.0,
            border_color: Some(Color::rgba(60, 60, 100, 180)),
            border_width: 1.0,
            interactive: false,
        },
    );

    hud.show();

    // Give the window time to appear
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Custom draw: bar chart with labels
    let bar_colors = [
        Color::rgba(80, 160, 255, 255),
        Color::rgba(100, 220, 160, 255),
        Color::rgba(255, 180, 80, 255),
        Color::rgba(255, 100, 120, 255),
    ];
    let bar_values: [f32; 4] = [0.7, 0.45, 0.9, 0.3];
    let bar_labels = ["Mon", "Tue", "Wed", "Thu"];

    let mut ops = Vec::new();

    // Title
    ops.push(DrawOp::DrawText {
        x: 20.0,
        y: 15.0,
        text: "Weekly Activity".into(),
        font_size: 18.0,
        color: Color::WHITE,
    });

    // Horizontal baseline
    ops.push(DrawOp::DrawLine {
        x1: 40.0,
        y1: 240.0,
        x2: 370.0,
        y2: 240.0,
        color: Color::rgba(80, 80, 120, 200),
        stroke_width: 1.0,
    });

    // Bars
    let bar_width = 60.0;
    let bar_max_height = 170.0;
    let start_x = 55.0;
    let spacing = 80.0;

    for (i, (&value, &color)) in bar_values.iter().zip(bar_colors.iter()).enumerate() {
        let x = start_x + i as f32 * spacing;
        let bar_height = value * bar_max_height;
        let y = 240.0 - bar_height;

        ops.push(DrawOp::FillRoundedRect {
            x,
            y,
            width: bar_width,
            height: bar_height,
            radius: 4.0,
            color,
        });

        // Label below bar
        ops.push(DrawOp::DrawText {
            x: x + 15.0,
            y: 248.0,
            text: bar_labels[i].into(),
            font_size: 12.0,
            color: Color::rgba(160, 160, 180, 255),
        });

        // Value above bar
        ops.push(DrawOp::DrawText {
            x: x + 12.0,
            y: y - 20.0,
            text: format!("{}%", (value * 100.0) as u32),
            font_size: 11.0,
            color,
        });
    }

    // Decorative ellipse
    ops.push(DrawOp::StrokeEllipse {
        cx: 370.0,
        cy: 30.0,
        rx: 12.0,
        ry: 12.0,
        color: Color::rgba(100, 180, 255, 120),
        stroke_width: 1.5,
    });

    hud.custom_draw(ops);

    println!("winpane custom_draw: overlay with bar chart at (200, 200).");
    println!("Press Ctrl+C to exit.");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

Register the example in `crates/winpane/Cargo.toml` (add after line 25):
```toml
[[example]]
name = "custom_draw"
path = "../../examples/rust/custom_draw.rs"
```

**Checkpoint:** At this point, `cargo build --workspace` and `cargo clippy --workspace` should pass (on Windows CI). The Rust custom draw API is complete.

---

## Phase 3: FFI crate setup

### 3.1 Update winpane-ffi Cargo.toml

**File:** `crates/winpane-ffi/Cargo.toml`

Replace entire file:
```toml
[package]
name = "winpane-ffi"
version = "0.0.1"
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "C ABI bindings for winpane - Windows companion surface SDK"

[lints]
workspace = true

[lib]
name = "winpane"
crate-type = ["cdylib", "staticlib"]

[dependencies]
winpane = { version = "0.0.1", path = "../winpane" }

[build-dependencies]
cbindgen = "0.29"
```

Notes:
- `cdylib` produces `winpane.dll` + `winpane.lib` on Windows.
- `staticlib` is included for consumers who want static linking.
- No `libc` dependency needed; use `std::os::raw::{c_char, c_int}` from std.
- `cbindgen = "0.29"` is the latest stable version.

### 3.2 Create cbindgen.toml

**File:** `crates/winpane-ffi/cbindgen.toml` (create)

```toml
language = "C"
header = "/* Generated by cbindgen - do not edit manually. */"
include_guard = "WINPANE_H"
tab_width = 4
style = "both"
cpp_compat = true

# Naming
[defines]

[export]
prefix = "WINPANE_"
include = []
exclude = []

[export.rename]
"WinpaneColor" = "winpane_color_t"
"WinpaneHudConfig" = "winpane_hud_config_t"
"WinpanePanelConfig" = "winpane_panel_config_t"
"WinpaneTrayConfig" = "winpane_tray_config_t"
"WinpaneTextElement" = "winpane_text_element_t"
"WinpaneRectElement" = "winpane_rect_element_t"
"WinpaneImageElement" = "winpane_image_element_t"
"WinpaneMenuItem" = "winpane_menu_item_t"
"WinpaneEventType" = "winpane_event_type_t"
"WinpaneMouseButton" = "winpane_mouse_button_t"
"WinpaneEvent" = "winpane_event_t"

[enum]
rename_variants = "ScreamingSnakeCase"
prefix_with_name = true

[fn]
# Functions use snake_case winpane_ prefix in Rust source
rename_args = "None"

[struct]
rename_fields = "None"
```

### 3.3 Create build.rs

**File:** `crates/winpane-ffi/build.rs` (create)

```rust
fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let config = cbindgen::Config::from_file(format!("{crate_dir}/cbindgen.toml"))
        .expect("failed to read cbindgen.toml");

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("cbindgen failed to generate header")
        .write_to_file("include/winpane.h");
}
```

Create the output directory: `crates/winpane-ffi/include/` (mkdir).

**Important:** cbindgen may fail if the crate doesn't compile yet. The build.rs runs at build time. Ensure the FFI source compiles before expecting the header to generate. If cbindgen can't parse the crate, wrap the generate call in an `if let Ok(...)` with a warning during initial development, then make it strict before merging.

---

## Phase 4: FFI error handling infrastructure

### 4.1 Replace lib.rs stub with error handling foundation

**File:** `crates/winpane-ffi/src/lib.rs`

Replace the 2-line stub with the error handling infrastructure. This is the foundation that all FFI functions depend on.

```rust
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

// Variant for functions that return a value through an out-pointer
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

// --- Null pointer validation helper ---

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

unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("string pointer is null".into());
    }
    // Safety: caller guarantees valid null-terminated UTF-8
    CStr::from_ptr(ptr)
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|e| format!("invalid UTF-8: {e}"))
}
```

---

## Phase 5: C-compatible type definitions

### 5.1 Add repr(C) types

**File:** `crates/winpane-ffi/src/lib.rs` (append after the helpers from Phase 4)

Define all `#[repr(C)]` types. These are what cbindgen sees and includes in the header.

```rust
// ============================================================
// C-compatible type definitions
// ============================================================

/// WINPANE_CONFIG_VERSION: consumers set this in config structs.
pub const WINPANE_CONFIG_VERSION: u32 = 1;

// --- Color ---

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WinpaneColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl WinpaneColor {
    fn to_rust(&self) -> winpane::Color {
        winpane::Color::rgba(self.r, self.g, self.b, self.a)
    }
}

// --- Config structs (versioned) ---

#[repr(C)]
pub struct WinpaneHudConfig {
    pub version: u32,
    pub size: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl WinpaneHudConfig {
    fn to_rust(&self) -> Result<winpane::HudConfig, String> {
        if self.version != WINPANE_CONFIG_VERSION {
            return Err(format!(
                "unsupported config version {} (expected {})",
                self.version, WINPANE_CONFIG_VERSION
            ));
        }
        Ok(winpane::HudConfig {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        })
    }
}

#[repr(C)]
pub struct WinpanePanelConfig {
    pub version: u32,
    pub size: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub draggable: i32,
    pub drag_height: u32,
}

impl WinpanePanelConfig {
    fn to_rust(&self) -> Result<winpane::PanelConfig, String> {
        if self.version != WINPANE_CONFIG_VERSION {
            return Err(format!(
                "unsupported config version {} (expected {})",
                self.version, WINPANE_CONFIG_VERSION
            ));
        }
        Ok(winpane::PanelConfig {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            draggable: self.draggable != 0,
            drag_height: self.drag_height,
        })
    }
}

#[repr(C)]
pub struct WinpaneTrayConfig {
    pub version: u32,
    pub size: u32,
    pub icon_rgba: *const u8,
    pub icon_rgba_len: u32,
    pub icon_width: u32,
    pub icon_height: u32,
    pub tooltip: *const c_char,
}

impl WinpaneTrayConfig {
    unsafe fn to_rust(&self) -> Result<winpane::TrayConfig, String> {
        if self.version != WINPANE_CONFIG_VERSION {
            return Err(format!(
                "unsupported config version {} (expected {})",
                self.version, WINPANE_CONFIG_VERSION
            ));
        }
        require_non_null(self.icon_rgba, "icon_rgba")?;
        let icon_data = std::slice::from_raw_parts(self.icon_rgba, self.icon_rgba_len as usize);
        let tooltip = cstr_to_string(self.tooltip)?;
        Ok(winpane::TrayConfig {
            icon_rgba: icon_data.to_vec(),
            icon_width: self.icon_width,
            icon_height: self.icon_height,
            tooltip,
        })
    }
}

// --- Element structs (value types, frozen per major version) ---

#[repr(C)]
pub struct WinpaneTextElement {
    pub text: *const c_char,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    pub color: WinpaneColor,
    pub font_family: *const c_char, // NULL for system default
    pub bold: i32,
    pub italic: i32,
    pub interactive: i32,
}

impl WinpaneTextElement {
    unsafe fn to_rust(&self) -> Result<winpane::TextElement, String> {
        let text = cstr_to_string(self.text)?;
        let font_family = if self.font_family.is_null() {
            None
        } else {
            Some(cstr_to_string(self.font_family)?)
        };
        Ok(winpane::TextElement {
            text,
            x: self.x,
            y: self.y,
            font_size: self.font_size,
            color: self.color.to_rust(),
            font_family,
            bold: self.bold != 0,
            italic: self.italic != 0,
            interactive: self.interactive != 0,
        })
    }
}

#[repr(C)]
pub struct WinpaneRectElement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fill: WinpaneColor,
    pub corner_radius: f32,
    pub has_border: i32,
    pub border_color: WinpaneColor,
    pub border_width: f32,
    pub interactive: i32,
}

impl WinpaneRectElement {
    fn to_rust(&self) -> winpane::RectElement {
        let border_color = if self.has_border != 0 {
            Some(self.border_color.to_rust())
        } else {
            None
        };
        winpane::RectElement {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            fill: self.fill.to_rust(),
            corner_radius: self.corner_radius,
            border_color,
            border_width: self.border_width,
            interactive: self.interactive != 0,
        }
    }
}

#[repr(C)]
pub struct WinpaneImageElement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub data: *const u8,
    pub data_len: u32,
    pub data_width: u32,
    pub data_height: u32,
    pub interactive: i32,
}

impl WinpaneImageElement {
    unsafe fn to_rust(&self) -> Result<winpane::ImageElement, String> {
        require_non_null(self.data, "image data")?;
        let data = std::slice::from_raw_parts(self.data, self.data_len as usize);
        Ok(winpane::ImageElement {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            data: data.to_vec(),
            data_width: self.data_width,
            data_height: self.data_height,
            interactive: self.interactive != 0,
        })
    }
}

// --- Menu item ---

#[repr(C)]
pub struct WinpaneMenuItem {
    pub id: u32,
    pub label: *const c_char,
    pub enabled: i32,
}

// --- Event ---

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinpaneEventType {
    None = 0,
    ElementClicked = 1,
    ElementHovered = 2,
    ElementLeft = 3,
    TrayClicked = 4,
    TrayMenuItemClicked = 5,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinpaneMouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

#[repr(C)]
pub struct WinpaneEvent {
    pub event_type: WinpaneEventType,
    pub surface_id: u64,
    pub key: [u8; 256], // null-terminated UTF-8
    pub mouse_button: WinpaneMouseButton,
    pub menu_item_id: u32,
}

impl WinpaneEvent {
    fn from_rust(event: &winpane::Event) -> Self {
        let mut e = WinpaneEvent {
            event_type: WinpaneEventType::None,
            surface_id: 0,
            key: [0u8; 256],
            mouse_button: WinpaneMouseButton::Left,
            menu_item_id: 0,
        };
        match event {
            winpane::Event::ElementClicked { surface_id, key } => {
                e.event_type = WinpaneEventType::ElementClicked;
                e.surface_id = surface_id.0;
                copy_key_to_buffer(key, &mut e.key);
            }
            winpane::Event::ElementHovered { surface_id, key } => {
                e.event_type = WinpaneEventType::ElementHovered;
                e.surface_id = surface_id.0;
                copy_key_to_buffer(key, &mut e.key);
            }
            winpane::Event::ElementLeft { surface_id, key } => {
                e.event_type = WinpaneEventType::ElementLeft;
                e.surface_id = surface_id.0;
                copy_key_to_buffer(key, &mut e.key);
            }
            winpane::Event::TrayClicked { button } => {
                e.event_type = WinpaneEventType::TrayClicked;
                e.mouse_button = match button {
                    winpane::MouseButton::Left => WinpaneMouseButton::Left,
                    winpane::MouseButton::Right => WinpaneMouseButton::Right,
                    winpane::MouseButton::Middle => WinpaneMouseButton::Middle,
                };
            }
            winpane::Event::TrayMenuItemClicked { id } => {
                e.event_type = WinpaneEventType::TrayMenuItemClicked;
                e.menu_item_id = *id;
            }
        }
        e
    }
}

fn copy_key_to_buffer(key: &str, buf: &mut [u8; 256]) {
    let bytes = key.as_bytes();
    let len = bytes.len().min(255); // leave room for null terminator
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
}
```

---

## Phase 6: Opaque handle types and context/event functions

### 6.1 Define opaque handles and FfiSurface enum

**File:** `crates/winpane-ffi/src/lib.rs` (append)

```rust
// ============================================================
// Opaque handle types (NOT #[repr(C)] - cbindgen generates forward declarations)
// ============================================================

/// Internal surface wrapper that unifies Hud and Panel behind one handle.
enum FfiSurface {
    Hud(winpane::Hud),
    Panel(winpane::Panel),
}

impl FfiSurface {
    fn id(&self) -> winpane::SurfaceId {
        match self {
            FfiSurface::Hud(h) => h.id(),
            FfiSurface::Panel(p) => p.id(),
        }
    }

    fn set_text(&self, key: &str, elem: winpane::TextElement) {
        match self {
            FfiSurface::Hud(h) => h.set_text(key, elem),
            FfiSurface::Panel(p) => p.set_text(key, elem),
        }
    }

    fn set_rect(&self, key: &str, elem: winpane::RectElement) {
        match self {
            FfiSurface::Hud(h) => h.set_rect(key, elem),
            FfiSurface::Panel(p) => p.set_rect(key, elem),
        }
    }

    fn set_image(&self, key: &str, elem: winpane::ImageElement) {
        match self {
            FfiSurface::Hud(h) => h.set_image(key, elem),
            FfiSurface::Panel(p) => p.set_image(key, elem),
        }
    }

    fn remove(&self, key: &str) {
        match self {
            FfiSurface::Hud(h) => h.remove(key),
            FfiSurface::Panel(p) => p.remove(key),
        }
    }

    fn show(&self) {
        match self {
            FfiSurface::Hud(h) => h.show(),
            FfiSurface::Panel(p) => p.show(),
        }
    }

    fn hide(&self) {
        match self {
            FfiSurface::Hud(h) => h.hide(),
            FfiSurface::Panel(p) => p.hide(),
        }
    }

    fn set_position(&self, x: i32, y: i32) {
        match self {
            FfiSurface::Hud(h) => h.set_position(x, y),
            FfiSurface::Panel(p) => p.set_position(x, y),
        }
    }

    fn set_size(&self, width: u32, height: u32) {
        match self {
            FfiSurface::Hud(h) => h.set_size(width, height),
            FfiSurface::Panel(p) => p.set_size(width, height),
        }
    }

    fn set_opacity(&self, opacity: f32) {
        match self {
            FfiSurface::Hud(h) => h.set_opacity(opacity),
            FfiSurface::Panel(p) => p.set_opacity(opacity),
        }
    }

    fn custom_draw(&self, ops: Vec<winpane::DrawOp>) {
        match self {
            FfiSurface::Hud(h) => h.custom_draw(ops),
            FfiSurface::Panel(p) => p.custom_draw(ops),
        }
    }
}

pub struct WinpaneContext {
    inner: winpane::Context,
}

pub struct WinpaneSurface {
    inner: FfiSurface,
    /// Active canvas for custom draw (one at a time per surface).
    canvas: Option<Box<CanvasAccumulator>>,
}

pub struct WinpaneTray {
    inner: winpane::Tray,
}

struct CanvasAccumulator {
    ops: Vec<winpane::DrawOp>,
}

pub struct WinpaneCanvas {
    ops: *mut Vec<winpane::DrawOp>,
}
```

### 6.2 Context lifecycle functions

**File:** `crates/winpane-ffi/src/lib.rs` (append)

```rust
// ============================================================
// Context lifecycle
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_create(out: *mut *mut WinpaneContext) -> i32 {
    ffi_try!({
        require_non_null_mut(out, "out")?;
        let ctx = winpane::Context::new().map_err(|e| e.to_string())?;
        let boxed = Box::new(WinpaneContext { inner: ctx });
        unsafe { *out = Box::into_raw(boxed) };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_destroy(ctx: *mut WinpaneContext) {
    if !ctx.is_null() {
        // Safety: ctx was created by winpane_create via Box::into_raw
        let _ = unsafe { Box::from_raw(ctx) };
    }
}

// ============================================================
// Event polling
// ============================================================

/// Polls for the next event. Returns 0 if an event was available
/// (event struct filled), 1 if no event pending, -1/-2 on error/panic.
#[no_mangle]
pub unsafe extern "C" fn winpane_poll_event(
    ctx: *mut WinpaneContext,
    event: *mut WinpaneEvent,
) -> i32 {
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        require_non_null(ctx, "ctx")?;
        require_non_null_mut(event, "event")?;
        let ctx = unsafe { &*ctx };
        match ctx.inner.poll_event() {
            Some(e) => {
                unsafe { *event = WinpaneEvent::from_rust(&e) };
                Ok(true) // event available
            }
            None => {
                unsafe { (*event).event_type = WinpaneEventType::None };
                Ok(false) // no event
            }
        }
    })) {
        Ok(Ok(true)) => 0_i32,  // event available
        Ok(Ok(false)) => 1_i32, // no event pending
        Ok(Err(e)) => {
            set_last_error(&e);
            -1_i32
        }
        Err(_) => {
            set_last_error("panic caught at FFI boundary");
            -2_i32
        }
    }
}
```

---

## Phase 7: Surface creation and operations

### 7.1 Surface creation functions

**File:** `crates/winpane-ffi/src/lib.rs` (append)

```rust
// ============================================================
// Surface creation
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_hud_create(
    ctx: *mut WinpaneContext,
    config: *const WinpaneHudConfig,
    out: *mut *mut WinpaneSurface,
) -> i32 {
    ffi_try!({
        require_non_null(ctx, "ctx")?;
        require_non_null(config, "config")?;
        require_non_null_mut(out, "out")?;
        let ctx = unsafe { &*ctx };
        let cfg = unsafe { &*config }.to_rust()?;
        let hud = ctx.inner.create_hud(cfg).map_err(|e| e.to_string())?;
        let surface = Box::new(WinpaneSurface {
            inner: FfiSurface::Hud(hud),
            canvas: None,
        });
        unsafe { *out = Box::into_raw(surface) };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_panel_create(
    ctx: *mut WinpaneContext,
    config: *const WinpanePanelConfig,
    out: *mut *mut WinpaneSurface,
) -> i32 {
    ffi_try!({
        require_non_null(ctx, "ctx")?;
        require_non_null(config, "config")?;
        require_non_null_mut(out, "out")?;
        let ctx = unsafe { &*ctx };
        let cfg = unsafe { &*config }.to_rust()?;
        let panel = ctx.inner.create_panel(cfg).map_err(|e| e.to_string())?;
        let surface = Box::new(WinpaneSurface {
            inner: FfiSurface::Panel(panel),
            canvas: None,
        });
        unsafe { *out = Box::into_raw(surface) };
        Ok(())
    })
}
```

### 7.2 Surface operations (11 functions)

**File:** `crates/winpane-ffi/src/lib.rs` (append)

```rust
// ============================================================
// Surface operations (unified for Hud and Panel)
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_destroy(surface: *mut WinpaneSurface) {
    if !surface.is_null() {
        let _ = unsafe { Box::from_raw(surface) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_id(surface: *const WinpaneSurface) -> u64 {
    if surface.is_null() {
        return 0;
    }
    unsafe { &*surface }.inner.id().0
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_text(
    surface: *mut WinpaneSurface,
    key: *const c_char,
    element: *const WinpaneTextElement,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(key, "key")?;
        require_non_null(element, "element")?;
        let surface = unsafe { &*surface };
        let key = unsafe { cstr_to_string(key)? };
        let elem = unsafe { &*element }.to_rust()?;
        surface.inner.set_text(&key, elem);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_rect(
    surface: *mut WinpaneSurface,
    key: *const c_char,
    element: *const WinpaneRectElement,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(key, "key")?;
        require_non_null(element, "element")?;
        let surface = unsafe { &*surface };
        let key = unsafe { cstr_to_string(key)? };
        let elem = unsafe { &*element }.to_rust();
        surface.inner.set_rect(&key, elem);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_image(
    surface: *mut WinpaneSurface,
    key: *const c_char,
    element: *const WinpaneImageElement,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(key, "key")?;
        require_non_null(element, "element")?;
        let surface = unsafe { &*surface };
        let key = unsafe { cstr_to_string(key)? };
        let elem = unsafe { &*element }.to_rust()?;
        surface.inner.set_image(&key, elem);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_remove(
    surface: *mut WinpaneSurface,
    key: *const c_char,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null(key, "key")?;
        let surface = unsafe { &*surface };
        let key = unsafe { cstr_to_string(key)? };
        surface.inner.remove(&key);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_show(surface: *mut WinpaneSurface) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.show();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_hide(surface: *mut WinpaneSurface) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.hide();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_position(
    surface: *mut WinpaneSurface,
    x: i32,
    y: i32,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.set_position(x, y);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_size(
    surface: *mut WinpaneSurface,
    width: u32,
    height: u32,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.set_size(width, height);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_set_opacity(
    surface: *mut WinpaneSurface,
    opacity: f32,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        unsafe { &*surface }.inner.set_opacity(opacity);
        Ok(())
    })
}
```

---

## Phase 8: Tray functions

**File:** `crates/winpane-ffi/src/lib.rs` (append)

```rust
// ============================================================
// Tray
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_create(
    ctx: *mut WinpaneContext,
    config: *const WinpaneTrayConfig,
    out: *mut *mut WinpaneTray,
) -> i32 {
    ffi_try!({
        require_non_null(ctx, "ctx")?;
        require_non_null(config, "config")?;
        require_non_null_mut(out, "out")?;
        let ctx = unsafe { &*ctx };
        let cfg = unsafe { &*config }.to_rust()?;
        let tray = ctx.inner.create_tray(cfg).map_err(|e| e.to_string())?;
        let boxed = Box::new(WinpaneTray { inner: tray });
        unsafe { *out = Box::into_raw(boxed) };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_destroy(tray: *mut WinpaneTray) {
    if !tray.is_null() {
        let _ = unsafe { Box::from_raw(tray) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_set_tooltip(
    tray: *mut WinpaneTray,
    tooltip: *const c_char,
) -> i32 {
    ffi_try!({
        require_non_null(tray, "tray")?;
        require_non_null(tooltip, "tooltip")?;
        let tray = unsafe { &*tray };
        let text = unsafe { cstr_to_string(tooltip)? };
        tray.inner.set_tooltip(&text);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_set_icon(
    tray: *mut WinpaneTray,
    rgba: *const u8,
    rgba_len: u32,
    width: u32,
    height: u32,
) -> i32 {
    ffi_try!({
        require_non_null(tray, "tray")?;
        require_non_null(rgba, "rgba")?;
        let tray = unsafe { &*tray };
        let data = unsafe { std::slice::from_raw_parts(rgba, rgba_len as usize) };
        tray.inner.set_icon(data.to_vec(), width, height);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_set_popup(
    tray: *mut WinpaneTray,
    panel: *const WinpaneSurface,
) -> i32 {
    ffi_try!({
        require_non_null(tray, "tray")?;
        require_non_null(panel, "panel")?;
        let tray = unsafe { &*tray };
        let surface = unsafe { &*panel };
        // set_popup requires a Panel reference in the Rust API.
        // Extract the Panel from the FfiSurface enum.
        match &surface.inner {
            FfiSurface::Panel(p) => {
                tray.inner.set_popup(p);
                Ok(())
            }
            FfiSurface::Hud(_) => Err("set_popup requires a panel surface, not a hud".into()),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_tray_set_menu(
    tray: *mut WinpaneTray,
    items: *const WinpaneMenuItem,
    count: u32,
) -> i32 {
    ffi_try!({
        require_non_null(tray, "tray")?;
        if count > 0 {
            require_non_null(items, "items")?;
        }
        let tray = unsafe { &*tray };
        let menu_items: Result<Vec<winpane::MenuItem>, String> = (0..count)
            .map(|i| {
                let item = unsafe { &*items.add(i as usize) };
                let label = unsafe { cstr_to_string(item.label)? };
                Ok(winpane::MenuItem {
                    id: item.id,
                    label,
                    enabled: item.enabled != 0,
                })
            })
            .collect();
        tray.inner.set_menu(menu_items?);
        Ok(())
    })
}
```

---

## Phase 9: Canvas functions (custom draw FFI)

**File:** `crates/winpane-ffi/src/lib.rs` (append)

```rust
// ============================================================
// Custom draw (canvas)
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_begin_draw(
    surface: *mut WinpaneSurface,
    out: *mut *mut WinpaneCanvas,
) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        require_non_null_mut(out, "out")?;
        let surface = unsafe { &mut *surface };
        if surface.canvas.is_some() {
            return Err("a canvas is already active on this surface; call end_draw first".into());
        }
        let mut acc = Box::new(CanvasAccumulator { ops: Vec::new() });
        let ops_ptr: *mut Vec<winpane::DrawOp> = &mut acc.ops;
        surface.canvas = Some(acc);
        let canvas = Box::new(WinpaneCanvas { ops: ops_ptr });
        unsafe { *out = Box::into_raw(canvas) };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_surface_end_draw(surface: *mut WinpaneSurface) -> i32 {
    ffi_try!({
        require_non_null(surface, "surface")?;
        let surface = unsafe { &mut *surface };
        let acc = surface
            .canvas
            .take()
            .ok_or_else(|| "no active canvas; call begin_draw first".to_string())?;
        surface.inner.custom_draw(acc.ops);
        // The WinpaneCanvas handle is now dangling. Consumers must not
        // use it after end_draw. Document this in the C header.
        Ok(())
    })
}

// --- Canvas drawing functions ---
// Each pushes a DrawOp to the accumulator.

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_clear(
    canvas: *mut WinpaneCanvas,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let canvas = unsafe { &mut *canvas };
        let ops = unsafe { &mut *canvas.ops };
        ops.push(winpane::DrawOp::Clear(color.to_rust()));
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_fill_rect(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::FillRect {
            x, y, width: w, height: h, color: color.to_rust(),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_stroke_rect(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: WinpaneColor,
    width: f32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::StrokeRect {
            x, y, width: w, height: h, color: color.to_rust(), stroke_width: width,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_draw_text(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    text: *const c_char,
    font_size: f32,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let text_str = unsafe { cstr_to_string(text)? };
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::DrawText {
            x, y, text: text_str, font_size, color: color.to_rust(),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_draw_line(
    canvas: *mut WinpaneCanvas,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    color: WinpaneColor,
    width: f32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::DrawLine {
            x1, y1, x2, y2, color: color.to_rust(), stroke_width: width,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_fill_ellipse(
    canvas: *mut WinpaneCanvas,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::FillEllipse {
            cx, cy, rx, ry, color: color.to_rust(),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_stroke_ellipse(
    canvas: *mut WinpaneCanvas,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    color: WinpaneColor,
    width: f32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::StrokeEllipse {
            cx, cy, rx, ry, color: color.to_rust(), stroke_width: width,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_draw_image(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rgba: *const u8,
    rgba_len: u32,
    img_w: u32,
    img_h: u32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        require_non_null(rgba, "rgba")?;
        let data = unsafe { std::slice::from_raw_parts(rgba, rgba_len as usize) };
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::DrawImage {
            x, y, width: w, height: h,
            rgba: data.to_vec(), img_width: img_w, img_height: img_h,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_fill_rounded_rect(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: WinpaneColor,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::FillRoundedRect {
            x, y, width: w, height: h, radius, color: color.to_rust(),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn winpane_canvas_stroke_rounded_rect(
    canvas: *mut WinpaneCanvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: WinpaneColor,
    width: f32,
) -> i32 {
    ffi_try!({
        require_non_null(canvas, "canvas")?;
        let ops = unsafe { &mut *(*canvas).ops };
        ops.push(winpane::DrawOp::StrokeRoundedRect {
            x, y, width: w, height: h, radius, color: color.to_rust(), stroke_width: width,
        });
        Ok(())
    })
}
```

**Checkpoint:** At this point, `cargo build --workspace` should succeed. The full FFI layer is implemented. Run `cargo fmt --all` and `cargo clippy --workspace` (on Windows CI).

---

## Phase 10: C examples

### 10.1 examples/c/hello_hud.c

**File:** `examples/c/hello_hud.c` (create)

A minimal C program that creates a HUD overlay with text and rect elements, shows it, and polls for events in a loop. Include `<windows.h>` for `Sleep()`. Compile instructions in header comment.

Key structure:
```c
#include "winpane.h"
#include <stdio.h>
#include <windows.h>

int main(void) {
    // 1. Create context
    // 2. Create HUD with versioned config
    // 3. Add background rect, title text, value text
    // 4. Show surface
    // 5. Event loop: poll_event + Sleep(16)
    // 6. Cleanup: surface_destroy, destroy
}
```

### 10.2 examples/c/custom_draw.c

**File:** `examples/c/custom_draw.c` (create)

Demonstrates the canvas API: begin_draw, fill_rect, draw_text, draw_line, fill_ellipse, stroke_rounded_rect, end_draw. Draws a simple bar chart similar to the Rust custom_draw example.

### 10.3 examples/c/CMakeLists.txt

**File:** `examples/c/CMakeLists.txt` (create)

```cmake
cmake_minimum_required(VERSION 3.20)
project(winpane_examples C)

# Find winpane SDK
set(WINPANE_DIR "${CMAKE_SOURCE_DIR}/../../crates/winpane-ffi")
set(WINPANE_INCLUDE "${WINPANE_DIR}/include")
set(WINPANE_LIB_DIR "${CMAKE_SOURCE_DIR}/../../target/debug")

add_executable(hello_hud hello_hud.c)
target_include_directories(hello_hud PRIVATE ${WINPANE_INCLUDE})
target_link_directories(hello_hud PRIVATE ${WINPANE_LIB_DIR})
target_link_libraries(hello_hud winpane)

add_executable(custom_draw custom_draw.c)
target_include_directories(custom_draw PRIVATE ${WINPANE_INCLUDE})
target_link_directories(custom_draw PRIVATE ${WINPANE_LIB_DIR})
target_link_libraries(custom_draw winpane)
```

### 10.4 examples/c/build.bat

**File:** `examples/c/build.bat` (create)

Simple MSVC build script for consumers without CMake:
```bat
@echo off
set WINPANE=..\..\crates\winpane-ffi
set LIB_DIR=..\..\target\debug
cl /W4 /I %WINPANE%\include hello_hud.c /link /LIBPATH:%LIB_DIR% winpane.lib
cl /W4 /I %WINPANE%\include custom_draw.c /link /LIBPATH:%LIB_DIR% winpane.lib
```

Delete `examples/c/.gitkeep` since the directory now has real files.

---

## Phase 11: CI and header verification

### 11.1 Update ci.yml

**File:** `.github/workflows/ci.yml`

Add two steps after "Build":

```yaml
      - uses: ilammy/msvc-dev-cmd@v1

      - name: Verify C header
        run: |
          cl /c /W4 /I crates/winpane-ffi/include crates/winpane-ffi/include/winpane.h
        shell: cmd
```

Note: The `ilammy/msvc-dev-cmd@v1` action is required because `cl.exe` is not on PATH by default on `windows-latest` runners. The current CI does not include this action.

### 11.2 Generate winpane.def

**File:** `crates/winpane-ffi/include/winpane.def` (create)

List all 35 exported symbols:
```def
LIBRARY winpane
EXPORTS
    winpane_last_error
    winpane_create
    winpane_destroy
    winpane_hud_create
    winpane_panel_create
    winpane_surface_destroy
    winpane_surface_id
    winpane_surface_set_text
    winpane_surface_set_rect
    winpane_surface_set_image
    winpane_surface_remove
    winpane_surface_show
    winpane_surface_hide
    winpane_surface_set_position
    winpane_surface_set_size
    winpane_surface_set_opacity
    winpane_tray_create
    winpane_tray_destroy
    winpane_tray_set_tooltip
    winpane_tray_set_icon
    winpane_tray_set_popup
    winpane_tray_set_menu
    winpane_poll_event
    winpane_surface_begin_draw
    winpane_surface_end_draw
    winpane_canvas_clear
    winpane_canvas_fill_rect
    winpane_canvas_stroke_rect
    winpane_canvas_draw_text
    winpane_canvas_draw_line
    winpane_canvas_fill_ellipse
    winpane_canvas_stroke_ellipse
    winpane_canvas_draw_image
    winpane_canvas_fill_rounded_rect
    winpane_canvas_stroke_rounded_rect
```

---

## Phase 12: Update phases-progress.md

**File:** `_docs/initial-implementation/phases-progress.md`

Update the P3 row in the phase table to **Complete**. Add a "P3: C ABI & FFI (Complete)" section under "Completed Phase Notes" documenting:

- **What was built:** winpane-ffi cdylib with 35 extern "C" functions, auto-generated winpane.h via cbindgen, custom draw escape hatch (DrawOp pipeline), thread-local error handling, versioned config structs, unified surface handle.
- **Key files:** `crates/winpane-ffi/src/lib.rs`, `crates/winpane-ffi/cbindgen.toml`, `crates/winpane-ffi/build.rs`, `crates/winpane-ffi/include/winpane.h`, `crates/winpane-core/src/types.rs` (DrawOp), `examples/c/hello_hud.c`, `examples/c/custom_draw.c`, `examples/rust/custom_draw.rs`.
- **API surface:** 35 functions (1 error, 2 context, 2 surface creation, 11 surface ops, 6 tray, 1 event, 2 draw lifecycle, 10 canvas drawing).
- **Gotchas for P4:** Custom draw is in-process only, not available over IPC. The canvas handle is invalid after end_draw. DrawOp is fire-and-forget (scene graph changes overwrite custom draw content). Config struct versioning is forward-compatible but element structs are frozen.

---

## Potential Issues and Mitigations

### cbindgen may not generate the header perfectly on first try
- cbindgen needs careful tuning of `cbindgen.toml` to get the right type names, enum styles, and include/exclude rules.
- **Mitigation:** Build iteratively. Get the crate compiling first, then tune cbindgen output. The header can be manually adjusted as a fallback, but auto-generation is the goal.

### D2D_POINT_2F vs Vector2
- windows-rs 0.62 changed `D2D_POINT_2F` to `Vector2` from `windows_numerics`. The `DrawLine` and `FillEllipse` operations use `D2D_POINT_2F`.
- **Mitigation:** Check the existing `render_text` method for the correct point type import. Use whatever the existing code uses. The `D2D_POINT_2F` type is still available in `Direct2D::Common`.

### Canvas lifetime safety
- The `WinpaneCanvas` pointer becomes dangling after `end_draw`. There is no compile-time protection in C.
- **Mitigation:** Document clearly in the header. Set the canvas pointer to NULL in end_draw if we store it somewhere accessible. In practice, the begin/end pattern is well-understood by C programmers.

### catch_unwind and UnwindSafe
- `catch_unwind` requires the closure to be `UnwindSafe`. `AssertUnwindSafe` wrapper is used because FFI boundary functions must not panic regardless.
- **Mitigation:** Already handled by the `ffi_try!` macro using `AssertUnwindSafe`.

### Clippy lint: print_stdout
- The workspace has `clippy::print_stdout = "warn"`. Examples use `println!` intentionally.
- **Mitigation:** Add `#[allow(clippy::print_stdout)]` to example `main()` functions, matching the existing pattern in `hud_demo.rs`.

### Event key truncation
- The C event struct has a 256-byte key buffer. Keys longer than 255 bytes are truncated.
- **Mitigation:** Document the limit. Element keys in dev-tool UIs are short identifiers. 255 bytes is generous.
