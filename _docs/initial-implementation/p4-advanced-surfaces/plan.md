# P4: Advanced Surfaces - Implementation Plan

Adds PiP (Picture-in-Picture) thumbnail viewer, window anchoring, and capture exclusion to winpane. Uses DWM thumbnail APIs for live window previews, SetWinEventHook for tracking external window position/state, and SetWindowDisplayAffinity for capture exclusion.

Reference: `proposal.md` for architecture decisions, `initial-plan.md` for full implementation details.

## Phases

- [x] Phase 1: [Types, Commands, and Features](1-types-commands-features/spec.md) - PipConfig/SourceRect/Anchor types, 6 new Command variants, windows-rs feature flags
- [x] Phase 2: [Monitor and Capture](2-monitor-and-capture/spec.md) - WindowMonitor module (SetWinEventHook infra), capture exclusion utility (build number detection + SetWindowDisplayAffinity)
- [x] Phase 3: [Engine Integration](3-engine-integration/spec.md) - PiP surface creation with DWM thumbnails, anchor positioning, monitor event processing, PiP guards on existing handlers
- [x] Phase 4: [Public API and FFI](4-public-api-and-ffi/spec.md) - Pip struct, anchor_to/unanchor/set_capture_excluded on all surfaces, 6 new FFI functions, C types, .def file
- [x] Phase 5: [Examples and Verification](5-examples-and-verification/spec.md) - pip_viewer, anchored_companion, capture_excluded examples, fmt/clippy/build checks
