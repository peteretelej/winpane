# Threading Model

## Engine thread

winpane spawns a dedicated thread when `Context::new()` is called. This thread:

1. Creates a message-only control window (not visible, used only for `PostMessageW` wakeups)
2. Enters a `GetMessageW` loop that processes Win32 messages and drains pending commands
3. Owns all HWNDs, GPU devices, swap chains, and renderers
4. Runs until `Context` is dropped, which sends a `Shutdown` command and joins the thread

No Win32 window or GPU resource is ever accessed from the consumer thread.

## Command channel

The consumer communicates with the engine via an `mpsc::Sender<Command>`. Commands are fire-and-forget for most operations (set element, show, hide, reposition). The exception is surface creation, which uses a oneshot reply channel so the consumer can block until the engine returns a `SurfaceId`.

After sending a command, the consumer calls `PostMessageW` on the control window to wake the engine's `GetMessageW` loop. Without this wake, commands would only be processed when the next Win32 message arrives (mouse move, timer, etc.).

## Event delivery

Events flow from the engine to the consumer through a separate `mpsc::Receiver<Event>`. The consumer polls with `Context::poll_event()`, which calls `try_recv()`. There are no callbacks, no async streams, and no blocking waits. The consumer drives their own loop and checks for events at whatever frequency they want (typically every 16ms for 60fps responsiveness).

## Thread-local queues

Win32 window procedures run on the engine thread but execute synchronously inside `DispatchMessageW`. They cannot directly modify engine state mid-dispatch. Instead, they write to thread-local queues:

- `PENDING_DPI_CHANGES` - `WM_DPICHANGED` events queued as `DpiChangeEvent`
- `PENDING_TRAY_EVENTS` - Tray icon notifications (clicks, menu selections)
- `PENDING_FADE_COMPLETIONS` - Timer-based fade animation completions

After each `GetMessageW` / `DispatchMessageW` cycle, the engine drains all three queues and processes the events (resize swap chains, emit user events, finalize animations).

## Shutdown sequence

1. `Context::drop` sends `Command::Shutdown`
2. Engine receives the command, exits the message loop
3. All surfaces are destroyed (windows closed, GPU resources freed)
4. The engine thread returns; `Context::drop` joins it

If the consumer thread panics or exits without dropping `Context`, the engine thread detects the broken channel and shuts down on its own.

## Thread safety

`Context` is `Send` but the surface handles (`Hud`, `Panel`, `Pip`, `Tray`) are also `Send` because they only hold an `mpsc::Sender` clone and a control HWND (wrapped in a `Send`-safe newtype). You can move surface handles to other threads. All operations are serialized through the command channel.
