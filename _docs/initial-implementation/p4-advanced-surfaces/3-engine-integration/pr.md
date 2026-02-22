integrate pip, anchoring, and capture exclusion into engine

The engine now handles PiP surface creation with DWM thumbnails, window
anchoring with position tracking and minimize/restore behavior, and
capture exclusion via SetWindowDisplayAffinity. Monitor events are
drained each loop iteration to reposition anchored surfaces and detect
closed source/target windows.

- Existing handlers (SetElement, RemoveElement, CustomDraw, SetOpacity,
  SetSize, DestroySurface, render loop, shutdown) branch on PiP
- Non-Windows stubs added for set_capture_excluded in window.rs
