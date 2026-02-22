# Phase 1: Custom Draw Core and Rust API

## Overview

Add the `DrawOp` type and `Command::CustomDraw` to winpane-core, implement draw op execution on the D2D render target, expose `custom_draw()` on the public `Hud` and `Panel` types, and create a Rust example. This is the foundation the FFI canvas API depends on.

## Prerequisites

- Read `initial-plan.md` Phases 1-2 for detailed code
- Read `proposal.md` "Custom draw pipeline" section
- Current source files: `crates/winpane-core/src/types.rs`, `command.rs`, `renderer.rs`, `engine.rs`, `lib.rs`; `crates/winpane/src/lib.rs`, `Cargo.toml`

## What to Build

### 1. DrawOp enum in types.rs

**File:** `crates/winpane-core/src/types.rs`

Add after the `ImageElement` struct (after line 118), before `HudConfig`:

```rust
/// Low-level drawing operations for the custom draw escape hatch.
/// Accumulated by the FFI canvas and sent as a batch to the engine.
#[derive(Debug, Clone)]
pub enum DrawOp {
    Clear(Color),
    FillRect { x: f32, y: f32, width: f32, height: f32, color: Color },
    StrokeRect { x: f32, y: f32, width: f32, height: f32, color: Color, stroke_width: f32 },
    DrawText { x: f32, y: f32, text: String, font_size: f32, color: Color },
    DrawLine { x1: f32, y1: f32, x2: f32, y2: f32, color: Color, stroke_width: f32 },
    FillEllipse { cx: f32, cy: f32, rx: f32, ry: f32, color: Color },
    StrokeEllipse { cx: f32, cy: f32, rx: f32, ry: f32, color: Color, stroke_width: f32 },
    DrawImage { x: f32, y: f32, width: f32, height: f32, rgba: Vec<u8>, img_width: u32, img_height: u32 },
    FillRoundedRect { x: f32, y: f32, width: f32, height: f32, radius: f32, color: Color },
    StrokeRoundedRect { x: f32, y: f32, width: f32, height: f32, radius: f32, color: Color, stroke_width: f32 },
}
```

10 variants covering: clear, fill/stroke rect, text, line, fill/stroke ellipse, image, fill/stroke rounded rect.

### 2. Command::CustomDraw in command.rs

**File:** `crates/winpane-core/src/command.rs`

Add `DrawOp` to the import line (line 4):
```rust
use crate::types::{DrawOp, Error, HudConfig, MenuItem, PanelConfig, SurfaceId, TrayConfig, TrayId};
```

Add new variant after `DestroyTray(TrayId)` (after line 67):
```rust
    // --- P3 commands ---
    CustomDraw {
        surface: SurfaceId,
        ops: Vec<DrawOp>,
    },
```

### 3. execute_draw_ops in renderer.rs

**File:** `crates/winpane-core/src/renderer.rs`

Add `Color` and `DrawOp` to the existing `use crate::types::...` import (line 11).

Add two methods on `SurfaceRenderer` after `set_opacity` (after line 490):

**`execute_draw_ops`** - public method that performs a full BeginDraw/EndDraw/Present cycle:
1. Release current target, get new back buffer from swapchain
2. Create bitmap from DXGI surface and set as target
3. BeginDraw, clear to transparent
4. Render retained-mode scene graph (iterate elements, call existing render_rect/render_text/render_image)
5. Execute each DrawOp via `execute_single_draw_op`
6. EndDraw, Present, DComposition Commit

Signature:
```rust
pub unsafe fn execute_draw_ops(
    &self,
    scene: &SceneGraph,
    gpu: &GpuResources,
    ops: &[DrawOp],
) -> Result<(), Error>
```

**`execute_single_draw_op`** - private method called between BeginDraw/EndDraw:
- `Clear(color)` -> `self.dc.Clear()`
- `FillRect` -> CreateSolidColorBrush + FillRectangle (scaled by DPI)
- `StrokeRect` -> CreateSolidColorBrush + DrawRectangle
- `DrawText` -> CreateTextFormat ("Segoe UI") + CreateSolidColorBrush + DrawText
- `DrawLine` -> CreateSolidColorBrush + DrawLine with D2D_POINT_2F
- `FillEllipse` -> CreateSolidColorBrush + FillEllipse with D2D1_ELLIPSE
- `StrokeEllipse` -> CreateSolidColorBrush + DrawEllipse
- `DrawImage` -> rgba_to_bgra + CreateBitmap + DrawBitmap
- `FillRoundedRect` -> CreateSolidColorBrush + FillRoundedRectangle
- `StrokeRoundedRect` -> CreateSolidColorBrush + DrawRoundedRectangle

All coordinates are scaled by `self.dpi_scale`. Reuse existing rendering patterns from `render_rect`, `render_text`, `render_image`.

See `initial-plan.md` Phase 1.3 for the full implementation code.

**D2D_POINT_2F note:** The existing renderer code does NOT use `D2D_POINT_2F` or `D2D1_ELLIPSE` anywhere (only `D2D_RECT_F`). `DrawLine` needs point types and `FillEllipse`/`StrokeEllipse` need `D2D1_ELLIPSE`. In windows-rs 0.62, check whether `D2D_POINT_2F` is available from `windows::Win32::Graphics::Direct2D::Common`. If not, use `windows_numerics::Vector2` or construct the struct inline. Reference the P0 gotchas note on this topic.

### 4. Handle CustomDraw in engine.rs

**File:** `crates/winpane-core/src/engine.rs`

In the command match block (around lines 166-305), add after the `Command::DestroyTray` arm:

```rust
Command::CustomDraw { surface, ops } => {
    if let Some(s) = surfaces.get(&surface) {
        let _ = s.renderer.execute_draw_ops(&s.scene, &gpu, &ops);
    }
}
```

No dirty flag needed - `execute_draw_ops` handles its own present cycle. The next scene graph change (via SetElement) will run the normal `render()` path and overwrite custom draw content. This is by design: custom draw is per-frame, not persistent.

### 5. Re-export DrawOp from lib.rs

**File:** `crates/winpane-core/src/lib.rs`

`DrawOp` is in `types.rs` which is `pub mod types` with `pub use types::*`. Since `DrawOp` is `pub`, it is automatically re-exported. Verify this compiles - no code change should be needed.

### 6. Public API: custom_draw() on Hud and Panel

**File:** `crates/winpane/src/lib.rs`

Add `DrawOp` to the re-export list (lines 5-8):
```rust
pub use winpane_core::{
    Color, DrawOp, Error, Event, HudConfig, ImageElement, MenuItem, MouseButton, PanelConfig,
    RectElement, SurfaceId, TextElement, TrayConfig, TrayId,
};
```

Add `custom_draw` method to `Hud` impl, after `id()` (after line 169):
```rust
    /// Execute custom draw operations on this surface.
    /// Renders the scene graph first, then the provided ops on top.
    /// One-shot: the next scene graph change overwrites custom draw content.
    pub fn custom_draw(&self, ops: Vec<DrawOp>) {
        self.send(Command::CustomDraw {
            surface: self.id,
            ops,
        });
    }
```

Add identical `custom_draw` method to `Panel` impl, after its `id()` (after line 258).

### 7. Rust custom draw example

**File:** `examples/rust/custom_draw.rs` (create)

A demo that creates a HUD, adds a retained-mode background rect, then uses `custom_draw()` to render a bar chart with labels on top. Uses `DrawOp::DrawText`, `DrawOp::FillRoundedRect`, `DrawOp::DrawLine`, `DrawOp::StrokeEllipse`.

See `initial-plan.md` Phase 2.2 for the full example code.

Register in `crates/winpane/Cargo.toml` (append after line 26):
```toml
[[example]]
name = "custom_draw"
path = "../../examples/rust/custom_draw.rs"
```

Add `#[allow(clippy::print_stdout)]` on `main()` to match existing example pattern.

## Connections

- **Previous phase:** P2 complete (Panel, Tray, events are working)
- **Next phase:** Phase 2 (FFI Crate Setup) depends on `DrawOp` and `Command::CustomDraw` existing in the workspace for the FFI crate to wrap

## Checkpoint

After this phase: `cargo build --workspace` and `cargo clippy --workspace` should pass. The Rust custom draw API is complete and ready for FFI wrapping.
