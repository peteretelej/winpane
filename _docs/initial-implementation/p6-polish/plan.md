# P6: Polish & Hardening

Production-harden the winpane SDK: add DWM backdrop effects, GPU device loss recovery, DPI hardening, fade animations, comprehensive documentation, and examples.

Reference: `proposal.md` for architecture decisions, `initial-plan.md` for detailed implementation notes.

## Phases

- [x] Phase 1: [Backdrop Effects](1-backdrop-effects/) - DWM Mica/Acrylic backdrop with version detection, all API layers
- [x] Phase 2: [Device Loss Recovery](2-device-loss-recovery/) - GPU device loss detection and automatic recovery from scene graph
- [ ] Phase 3: [DPI Hardening](3-dpi-hardening/) - Verify and fix WM_DPICHANGED handling, anchor offset scaling
- [ ] Phase 4: [Fade Animations](4-fade-animations/) - DirectComposition opacity animations with timer-based completion
- [ ] Phase 5: [Documentation](5-documentation/) - Architecture guide, cookbook, signing guide, limitations, protocol update, README rewrite
- [ ] Phase 6: [Examples & CI](6-examples-ci/) - New examples for P6 features, CI verification, pre-push checks
