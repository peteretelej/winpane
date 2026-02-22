# P3: C ABI & FFI - Learnings

Corrections, gotchas, and tips discovered during implementation. Read this before starting any phase.

## Phase 1

- **D2D point types:** Confirmed that `D2D_POINT_2F` is replaced by `windows_numerics::Vector2` in windows-rs 0.62. Used `Vector2 { X: ..., Y: ... }` for `DrawLine` and `D2D1_ELLIPSE.point`. The `windows-numerics = "0.3"` dependency was already present in winpane-core.
- **Color import in renderer.rs:** `Color` is not needed as a direct import in renderer.rs since it's only encountered through `DrawOp` pattern matching (the bound variables are `&Color` but the type is never referenced explicitly). Only `DrawOp` needs to be imported.
