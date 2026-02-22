add window monitor and capture exclusion infrastructure

WindowMonitor tracks external windows via SetWinEventHook for PiP source close detection and anchor position tracking. Capture exclusion uses RtlGetVersion for build detection and SetWindowDisplayAffinity with WDA_EXCLUDEFROMCAPTURE on Win10 2004+ (falls back to WDA_MONITOR on older builds).
