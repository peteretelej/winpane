# Phase 4: Proposed Commit

add directcomposition fade in/out animations

Smooth opacity fade animations using DirectComposition native animation objects
(IDCompositionAnimation + IDCompositionEffectGroup). DWM handles interpolation
at display refresh rate with zero CPU cost during animation.

- fade_out hides the window after animation via WM_TIMER one-shot callback
- show() resets opacity for previously faded-out surfaces
- Available across all API layers: Rust, C ABI, JSON-RPC host, Node.js addon
