use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::command::{Command, CommandReceiver, CommandSender};
use crate::display;
use crate::input::{HitTestMap, PanelState};
use crate::monitor::{MonitorEvent, PENDING_MONITOR_EVENTS, Watch, WatchReason, WindowMonitor};
use crate::persist;
use crate::renderer::{GpuResources, RenderError, SurfaceRenderer};
use crate::scene::SceneGraph;
use crate::tray::{
    TrayState, create_hicon_from_rgba, create_tray_icon, destroy_tray_icon, show_tray_context_menu,
    update_tray_icon, update_tray_tooltip,
};
use crate::types::{
    Anchor, Error, Event, HudConfig, MouseButton, PanelConfig, PipConfig, Placement, SourceRect,
    SurfaceId, TrayConfig, TrayId,
};
use crate::window::{
    PENDING_DPI_CHANGES, PENDING_FADE_COMPLETIONS, PENDING_POSITION_CHANGES, PENDING_TRAY_EVENTS,
    SendHwnd, create_control_window, create_hud_window, create_panel_window,
    ensure_classes_registered, get_dpi_scale, try_set_dpi_awareness,
};

use windows::Win32::Graphics::Dwm::{
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE,
    DWM_TNP_VISIBLE, DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
};

// --- SurfaceKind ---

pub(crate) enum SurfaceKind {
    Hud,
    Panel(Box<PanelState>),
    Pip(PipState),
}

pub(crate) struct PipState {
    /// DWM thumbnail handle (HTHUMBNAIL). Zero if invalid.
    pub thumbnail: isize,
    /// Source window HWND as isize.
    #[allow(dead_code)]
    pub source_hwnd: isize,
    /// Optional source crop region.
    pub source_region: Option<SourceRect>,
    /// Current opacity (0.0..1.0), tracked for thumbnail property updates.
    pub opacity: f32,
}

pub(crate) struct AnchorState {
    pub target_hwnd: isize,
    pub anchor: Anchor,
    pub offset: (i32, i32),
    pub was_visible_before_minimize: bool,
}

// --- Surface (internal to engine) ---

pub(crate) struct Surface {
    pub renderer: SurfaceRenderer,
    pub scene: SceneGraph,
    pub visible: bool,
    pub kind: SurfaceKind,
    pub opacity: f32,
    pub fading_out: bool,
    pub position_key: Option<String>,
}

// --- EngineHandle (returned to winpane crate) ---

pub struct EngineHandle {
    pub sender: CommandSender,
    pub control_hwnd: SendHwnd,
    pub join_handle: Option<std::thread::JoinHandle<()>>,
}

impl EngineHandle {
    pub fn spawn() -> Result<(Self, mpsc::Receiver<Event>), Error> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (event_tx, event_rx) = mpsc::channel::<Event>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<SendHwnd, Error>>();

        let handle = std::thread::Builder::new()
            .name("winpane-engine".into())
            .spawn(move || {
                // SAFETY: Engine thread entry with COM, window, and GPU initialization.
                unsafe { engine_thread_main(cmd_rx, event_tx, ready_tx) };
            })
            .map_err(|_| Error::ThreadSpawnFailed)?;

        // Block until the engine thread signals ready
        let control_hwnd = ready_rx.recv().map_err(|_| Error::ThreadSpawnFailed)??;

        Ok((
            EngineHandle {
                sender: cmd_tx,
                control_hwnd,
                join_handle: Some(handle),
            },
            event_rx,
        ))
    }

    /// Send a command and wake the engine thread. Silently ignores send errors
    /// (channel closed = shutdown).
    pub fn send_and_wake(&self, cmd: Command) {
        let _ = self.sender.send(cmd);
        self.wake();
    }

    /// Wake the engine thread's message loop via PostMessageW.
    pub fn wake(&self) {
        // SAFETY: PostMessageW is thread-safe; SendHwnd wraps HWND for cross-thread use.
        unsafe {
            let _ = PostMessageW(Some(self.control_hwnd.0), WM_APP, WPARAM(0), LPARAM(0));
        }
    }
}

/// Wake the engine thread's message loop from any thread.
/// This is a free function so that `Hud` (which only holds a `SendHwnd`)
/// can wake the engine without needing the full `EngineHandle`.
pub fn wake_engine(control_hwnd: SendHwnd) {
    // SAFETY: PostMessageW is thread-safe by Win32 specification.
    // SendHwnd wraps HWND to allow cross-thread use.
    unsafe {
        let _ = PostMessageW(Some(control_hwnd.0), WM_APP, WPARAM(0), LPARAM(0));
    }
}

// --- COM initialization guard ---

/// RAII guard that calls `CoUninitialize` on drop, ensuring COM is properly
/// cleaned up on all engine thread exit paths (normal shutdown, early return).
struct CoInitGuard;

impl Drop for CoInitGuard {
    fn drop(&mut self) {
        // SAFETY: CoUninitialize pairs with CoInitializeEx called on this thread.
        unsafe {
            CoUninitialize();
        }
    }
}

// --- Engine thread main ---

#[allow(clippy::print_stderr)]
unsafe fn engine_thread_main(
    cmd_rx: CommandReceiver,
    event_tx: mpsc::Sender<Event>,
    ready_tx: mpsc::Sender<Result<SendHwnd, Error>>,
) {
    // SAFETY: Engine thread entry with COM, window, and GPU initialization.
    unsafe {
        // 1. STA for message loop thread
        if let Err(e) = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() {
            let _ = ready_tx.send(Err(Error::DeviceCreation(format!("CoInitializeEx: {e}"))));
            return;
        }
        let _co_guard = CoInitGuard;

        // 2. DPI awareness
        try_set_dpi_awareness();

        // 3. Register window classes
        ensure_classes_registered();

        // 4. GPU resources
        let mut gpu = match GpuResources::new() {
            Ok(gpu) => gpu,
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };

        // 5. Control window (message-only, for PostMessage wakeups)
        let control_hwnd = match create_control_window() {
            Ok(hwnd) => hwnd,
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };

        // 6. Signal ready
        let _ = ready_tx.send(Ok(SendHwnd(control_hwnd)));

        // 7. Initialize surface storage
        let mut surfaces: HashMap<SurfaceId, Surface> = HashMap::new();
        let mut next_id = SurfaceId(1);

        // 8. Tray state
        let mut trays: HashMap<TrayId, TrayState> = HashMap::new();
        let mut next_tray_id = TrayId(1);

        // 8b. Window monitor and anchor state
        let mut monitor = WindowMonitor::new();
        let mut anchor_states: HashMap<SurfaceId, AnchorState> = HashMap::new();

        // 8c. Position save debounce state
        let mut pending_saves: HashMap<String, (i32, i32, Instant)> = HashMap::new();

        // 9. Message loop
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if !ret.as_bool() {
                break; // WM_QUIT
            }

            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            // Process pending DPI changes from wndproc
            process_dpi_changes(&gpu, &mut surfaces);

            // Process tray events from control_wndproc
            process_tray_events(&event_tx, &mut trays, &mut surfaces);

            // Process window monitor events (PiP source close, anchor position updates)
            process_monitor_events(&mut surfaces, &mut anchor_states, &mut monitor, &event_tx);

            // Process fade completion events
            process_fade_completions(&mut surfaces);

            // Process position change events (SurfaceMoved + debounced persistence)
            process_position_changes(&surfaces, &event_tx, &mut pending_saves);

            // Drain command queue (non-blocking)
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    // --- Surface lifecycle ---
                    Command::CreateHud { config, reply } => {
                        let result = create_hud_surface(&gpu, &mut surfaces, &mut next_id, config);
                        let _ = reply.send(result);
                    }
                    Command::CreatePanel { config, reply } => {
                        let result = create_panel_surface(
                            &gpu,
                            &mut surfaces,
                            &mut next_id,
                            config,
                            &event_tx,
                        );
                        let _ = reply.send(result);
                    }
                    Command::CreatePip { config, reply } => {
                        let result = create_pip_surface(
                            &config,
                            &gpu,
                            &mut surfaces,
                            &mut next_id,
                            &mut monitor,
                        );
                        let _ = reply.send(result);
                    }
                    Command::DestroySurface(id) => {
                        if let Some(surface) = surfaces.remove(&id) {
                            match &surface.kind {
                                SurfaceKind::Panel(_) => {
                                    let _ =
                                        SetWindowLongPtrW(surface.renderer.hwnd, GWLP_USERDATA, 0);
                                }
                                SurfaceKind::Pip(pip) => {
                                    let _ = DwmUnregisterThumbnail(pip.thumbnail);
                                    monitor.unwatch_surface(id);
                                }
                                SurfaceKind::Hud => {}
                            }
                            // Also remove anchor state if this surface was anchored
                            if anchor_states.remove(&id).is_some() {
                                monitor.unwatch_surface(id);
                            }
                            let _ = DestroyWindow(surface.renderer.hwnd);
                        }
                    }
                    Command::Shutdown => {
                        // Destroy all trays
                        for (_, state) in trays.drain() {
                            destroy_tray_icon(state.hwnd, state.icon_id, state.hicon);
                        }
                        // Destroy all surfaces
                        for (_, surface) in surfaces.drain() {
                            match &surface.kind {
                                SurfaceKind::Panel(_) => {
                                    let _ =
                                        SetWindowLongPtrW(surface.renderer.hwnd, GWLP_USERDATA, 0);
                                }
                                SurfaceKind::Pip(pip) => {
                                    let _ = DwmUnregisterThumbnail(pip.thumbnail);
                                }
                                SurfaceKind::Hud => {}
                            }
                            let _ = DestroyWindow(surface.renderer.hwnd);
                        }
                        let _ = DestroyWindow(control_hwnd);
                        return;
                    }

                    // --- Scene graph ---
                    Command::SetElement {
                        surface,
                        key,
                        element,
                    } => {
                        if let Some(s) = surfaces.get_mut(&surface) {
                            // PiP surfaces have no scene graph
                            if matches!(s.kind, SurfaceKind::Pip(_)) {
                                continue;
                            }
                            // Only rebuild hit-test map if the element is interactive
                            // or the previous element at this key was interactive.
                            // Non-interactive updates (e.g. changing text color every
                            // frame) skip the rebuild entirely.
                            let needs_hit_rebuild = element.is_interactive()
                                || s.scene
                                    .get(&key)
                                    .is_some_and(super::scene::Element::is_interactive);
                            s.scene.set(key, element);
                            if needs_hit_rebuild && let SurfaceKind::Panel(state) = &mut s.kind {
                                state.hit_test_map.rebuild(&s.scene, s.renderer.dpi_scale);
                            }
                        }
                    }
                    Command::RemoveElement { surface, key } => {
                        if let Some(s) = surfaces.get_mut(&surface) {
                            // PiP surfaces have no scene graph
                            if matches!(s.kind, SurfaceKind::Pip(_)) {
                                continue;
                            }
                            s.scene.remove(&key);
                            // Rebuild hit-test map for panels
                            if let SurfaceKind::Panel(state) = &mut s.kind {
                                state.hit_test_map.rebuild(&s.scene, s.renderer.dpi_scale);
                            }
                        }
                    }

                    // --- Surface properties ---
                    Command::Show(id) => {
                        if let Some(s) = surfaces.get_mut(&id) {
                            if s.fading_out {
                                let _ = s.renderer.set_opacity(&gpu.dcomp_device, 1.0);
                                s.opacity = 1.0;
                                s.fading_out = false;
                            }
                            let _ = ShowWindow(s.renderer.hwnd, SW_SHOWNOACTIVATE);
                            s.visible = true;
                            s.scene.set_dirty();
                        }
                    }
                    Command::Hide(id) => {
                        if let Some(s) = surfaces.get_mut(&id) {
                            let _ = ShowWindow(s.renderer.hwnd, SW_HIDE);
                            s.visible = false;
                        }
                    }
                    Command::SetPosition { surface, x, y } => {
                        if let Some(s) = surfaces.get(&surface) {
                            let scale = s.renderer.dpi_scale;
                            let _ = SetWindowPos(
                                s.renderer.hwnd,
                                None,
                                (x as f32 * scale) as i32,
                                (y as f32 * scale) as i32,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                            );
                        }
                    }
                    Command::SetSize {
                        surface,
                        width,
                        height,
                    } => {
                        if let Some(s) = surfaces.get_mut(&surface) {
                            let _ = s.renderer.resize(&gpu, width, height);
                            match &s.kind {
                                SurfaceKind::Pip(pip) => {
                                    update_pip_thumbnail_properties(
                                        s.renderer.hwnd,
                                        pip,
                                        &s.renderer,
                                    );
                                }
                                _ => {
                                    s.scene.set_dirty();
                                }
                            }
                        }
                    }
                    Command::SetOpacity { surface, opacity } => {
                        if let Some(s) = surfaces.get_mut(&surface) {
                            let clamped = opacity.clamp(0.0, 1.0);
                            match &mut s.kind {
                                SurfaceKind::Pip(pip) => {
                                    pip.opacity = clamped;
                                    update_pip_thumbnail_properties(
                                        s.renderer.hwnd,
                                        pip,
                                        &s.renderer,
                                    );
                                }
                                _ => {
                                    let _ = s.renderer.set_opacity(&gpu.dcomp_device, clamped);
                                }
                            }
                            s.opacity = clamped;
                        }
                    }
                    Command::SetBackdrop { surface, backdrop } => {
                        if let Some(s) = surfaces.get(&surface) {
                            crate::window::set_window_backdrop(s.renderer.hwnd, backdrop);
                        }
                    }
                    Command::FadeIn {
                        surface,
                        duration_ms,
                    } => {
                        if let Some(s) = surfaces.get_mut(&surface) {
                            if !s.visible {
                                let _ = ShowWindow(s.renderer.hwnd, SW_SHOWNOACTIVATE);
                                s.visible = true;
                                s.scene.set_dirty();
                            }
                            let _ = s.renderer.animate_opacity(
                                &gpu.dcomp_device,
                                0.0,
                                1.0,
                                duration_ms,
                            );
                            s.fading_out = false;
                            s.opacity = 1.0;
                        }
                    }
                    Command::FadeOut {
                        surface,
                        duration_ms,
                    } => {
                        if let Some(s) = surfaces.get_mut(&surface)
                            && s.visible
                        {
                            let _ = s.renderer.animate_opacity(
                                &gpu.dcomp_device,
                                s.opacity,
                                0.0,
                                duration_ms,
                            );
                            s.fading_out = true;
                            let _ = SetTimer(
                                Some(s.renderer.hwnd),
                                surface.0 as usize,
                                duration_ms,
                                None,
                            );
                        }
                    }

                    // --- Tray ---
                    Command::CreateTray { config, reply } => {
                        let result =
                            create_tray(&mut trays, &mut next_tray_id, control_hwnd, config);
                        let _ = reply.send(result);
                    }
                    Command::SetTrayTooltip { tray, tooltip } => {
                        if let Some(state) = trays.get(&tray) {
                            let _ = update_tray_tooltip(state.hwnd, state.icon_id, &tooltip);
                        }
                    }
                    Command::SetTrayIcon {
                        tray,
                        rgba,
                        width,
                        height,
                    } => {
                        if let Some(state) = trays.get_mut(&tray)
                            && let Ok(new_icon) = create_hicon_from_rgba(&rgba, width, height)
                        {
                            let old_icon = state.hicon;
                            state.hicon = new_icon;
                            let _ = update_tray_icon(state.hwnd, state.icon_id, new_icon);
                            let _ = DestroyIcon(old_icon);
                        }
                    }
                    Command::SetTrayPopup { tray, surface } => {
                        if let Some(state) = trays.get_mut(&tray) {
                            state.popup_surface = Some(surface);
                            state.popup_visible = false;
                        }
                    }
                    Command::SetTrayMenu { tray, items } => {
                        if let Some(state) = trays.get_mut(&tray) {
                            state.menu_items = items;
                        }
                    }
                    Command::DestroyTray(id) => {
                        if let Some(state) = trays.remove(&id) {
                            destroy_tray_icon(state.hwnd, state.icon_id, state.hicon);
                        }
                    }

                    // --- Custom draw ---
                    Command::CustomDraw { surface, ops } => {
                        if let Some(s) = surfaces.get(&surface) {
                            // PiP has no D2D rendering
                            if matches!(s.kind, SurfaceKind::Pip(_)) {
                                continue;
                            }
                            match s.renderer.execute_draw_ops(&s.scene, &gpu, &ops) {
                                Err(RenderError::DeviceLost) => {
                                    recover_device(&mut gpu, &mut surfaces, &event_tx);
                                }
                                Err(RenderError::Other(msg)) => {
                                    eprintln!("[winpane] custom draw error: {msg}");
                                }
                                Ok(()) => {}
                            }
                        }
                    }

                    // --- Window tracking ---
                    Command::SetSourceRegion { surface, rect } => {
                        if let Some(s) = surfaces.get_mut(&surface)
                            && let SurfaceKind::Pip(pip) = &mut s.kind
                        {
                            pip.source_region = Some(rect);
                            update_pip_thumbnail_properties(s.renderer.hwnd, pip, &s.renderer);
                        }
                    }
                    Command::ClearSourceRegion { surface } => {
                        if let Some(s) = surfaces.get_mut(&surface)
                            && let SurfaceKind::Pip(pip) = &mut s.kind
                        {
                            pip.source_region = None;
                            update_pip_thumbnail_properties(s.renderer.hwnd, pip, &s.renderer);
                        }
                    }
                    Command::AnchorTo {
                        surface,
                        target_hwnd,
                        anchor,
                        offset,
                    } => {
                        if surfaces.contains_key(&surface) {
                            // Remove previous anchor if any
                            if let Some(old) = anchor_states.remove(&surface) {
                                monitor.unwatch(old.target_hwnd, surface);
                            }

                            // Store new anchor state
                            anchor_states.insert(
                                surface,
                                AnchorState {
                                    target_hwnd,
                                    anchor,
                                    offset,
                                    was_visible_before_minimize: false,
                                },
                            );

                            // Register in monitor
                            monitor.watch(
                                target_hwnd,
                                surface,
                                WatchReason::AnchorTarget { anchor, offset },
                            );

                            // Immediate initial positioning
                            apply_anchor_position(&surfaces, &anchor_states, surface);
                        }
                    }
                    Command::Unanchor { surface } => {
                        if let Some(state) = anchor_states.remove(&surface) {
                            monitor.unwatch(state.target_hwnd, surface);
                        }
                    }
                    Command::SetCaptureExcluded { surface, excluded } => {
                        if let Some(s) = surfaces.get(&surface) {
                            crate::window::set_capture_excluded(s.renderer.hwnd, excluded);
                        }
                    }
                    Command::GetPosition { surface, reply } => {
                        let result = surfaces
                            .get(&surface)
                            .map(|s| {
                                let mut rect = RECT::default();
                                let _ = GetWindowRect(s.renderer.hwnd, &mut rect);
                                (rect.left, rect.top)
                            })
                            .ok_or(Error::SurfaceNotFound);
                        let _ = reply.send(result);
                    }
                }
            }

            // Render dirty visible surfaces
            let mut device_lost = false;
            for surface in surfaces.values_mut() {
                if surface.visible && surface.scene.take_dirty() {
                    // DWM handles PiP rendering
                    if matches!(surface.kind, SurfaceKind::Pip(_)) {
                        continue;
                    }
                    match surface.renderer.render(&surface.scene, &gpu) {
                        Err(RenderError::DeviceLost) => {
                            device_lost = true;
                            break;
                        }
                        Err(RenderError::Other(msg)) => {
                            eprintln!("[winpane] render error: {msg}");
                        }
                        Ok(()) => {}
                    }
                }
            }

            // Commit all DComp changes from this render pass atomically
            if !device_lost {
                let _ = gpu.dcomp_device.Commit();
            }

            if device_lost {
                recover_device(&mut gpu, &mut surfaces, &event_tx);
            }
        }
    } // unsafe
}

/// Attempt GPU device recovery after device loss.
/// Recreates GpuResources and all per-surface device-dependent resources,
/// then sends a DeviceRecovered event.
#[allow(clippy::print_stderr)]
unsafe fn recover_device(
    gpu: &mut GpuResources,
    surfaces: &mut HashMap<SurfaceId, Surface>,
    event_tx: &mpsc::Sender<Event>,
) {
    // SAFETY: GPU device recovery with valid device references.
    unsafe {
        // Log the reason for device removal
        let reason = gpu.d3d_device.GetDeviceRemovedReason();
        eprintln!("[winpane] device lost, removed reason: {reason:?}");

        // Recreate GPU resources
        let new_gpu = match GpuResources::new() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[winpane] device recovery failed: {e}");
                return;
            }
        };
        *gpu = new_gpu;

        // Recreate per-surface device resources
        let mut all_ok = true;
        for surface in surfaces.values_mut() {
            if let Err(e) = surface.renderer.create_device_resources(gpu) {
                eprintln!("[winpane] surface recovery failed: {e}");
                all_ok = false;
            } else {
                surface.scene.set_dirty();
            }
        }

        if all_ok {
            let _ = event_tx.send(Event::DeviceRecovered);
        }
    } // unsafe
}

// --- DPI change processing ---

fn process_dpi_changes(gpu: &GpuResources, surfaces: &mut HashMap<SurfaceId, Surface>) {
    PENDING_DPI_CHANGES.with(|changes| {
        for event in changes.borrow_mut().drain(..) {
            for surface in surfaces.values_mut() {
                if surface.renderer.hwnd == event.hwnd {
                    let new_scale = event.new_dpi as f32 / 96.0;
                    surface.renderer.dpi_scale = new_scale;
                    // SAFETY: Resize with valid GPU resources after DPI change.
                    let _ = unsafe {
                        surface.renderer.resize(
                            gpu,
                            surface.renderer.width,
                            surface.renderer.height,
                        )
                    };
                    surface.scene.set_dirty();

                    // Update panel state for new DPI
                    if let SurfaceKind::Panel(state) = &mut surface.kind {
                        state.hit_test_map.rebuild(&surface.scene, new_scale);
                        state.drag_height = state.logical_drag_height * new_scale;
                    }

                    break;
                }
            }
        }
    });
}

// --- Fade completion processing ---

unsafe fn process_fade_completions(surfaces: &mut HashMap<SurfaceId, Surface>) {
    // SAFETY: ShowWindow with HWND from engine-owned surfaces.
    unsafe {
        PENDING_FADE_COMPLETIONS.with(|completions| {
            for event in completions.borrow_mut().drain(..) {
                for surface in surfaces.values_mut() {
                    if surface.renderer.hwnd == event.hwnd {
                        surface.visible = false;
                        surface.opacity = 0.0;
                        let _ = ShowWindow(surface.renderer.hwnd, SW_HIDE);
                        break;
                    }
                }
            }
        });
    } // unsafe
}

// --- Tray event processing ---

/// Process tray icon notifications from the control window.
///
/// V1 limitation: only a single tray icon is supported. All notifications
/// are routed to the first (and only) tray in the map. The `TrayId` in
/// notifications is ignored; callers that create multiple trays will see
/// all events attributed to whichever tray happens to be first in iteration
/// order. This will be addressed when multi-tray support is added.
unsafe fn process_tray_events(
    event_tx: &mpsc::Sender<Event>,
    trays: &mut HashMap<TrayId, TrayState>,
    surfaces: &mut HashMap<SurfaceId, Surface>,
) {
    // SAFETY: Win32 tray/cursor/window APIs with engine-owned HWNDs.
    unsafe {
        debug_assert!(trays.len() <= 1, "winpane V1 supports at most one tray");
        PENDING_TRAY_EVENTS.with(|events| {
            for notification in events.borrow_mut().drain(..) {
                // Find the tray (for single-tray V1, there's at most one)
                let Some((_, tray_state)) = trays.iter_mut().next() else {
                    continue;
                };

                match notification.event {
                    // Left-click: toggle popup + send event
                    WM_LBUTTONUP => {
                        if let Some(surface_id) = tray_state.popup_surface
                            && let Some(surface) = surfaces.get_mut(&surface_id)
                        {
                            if tray_state.popup_visible {
                                let _ = ShowWindow(surface.renderer.hwnd, SW_HIDE);
                                surface.visible = false;
                                tray_state.popup_visible = false;
                            } else {
                                // Position near cursor
                                let mut cursor = POINT::default();
                                let _ = GetCursorPos(&mut cursor);
                                let scale = surface.renderer.dpi_scale;
                                let phys_h = (surface.renderer.height as f32 * scale) as i32;
                                let _ = SetWindowPos(
                                    surface.renderer.hwnd,
                                    None,
                                    cursor.x,
                                    cursor.y - phys_h,
                                    0,
                                    0,
                                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                                );
                                let _ = ShowWindow(surface.renderer.hwnd, SW_SHOWNOACTIVATE);
                                surface.visible = true;
                                surface.scene.set_dirty();
                                tray_state.popup_visible = true;
                            }
                        }
                        let _ = event_tx.send(Event::TrayClicked {
                            button: MouseButton::Left,
                        });
                    }

                    // Right-click: show context menu + send event
                    WM_RBUTTONUP => {
                        if !tray_state.menu_items.is_empty() {
                            let selected =
                                show_tray_context_menu(tray_state.hwnd, &tray_state.menu_items);
                            if selected > 0 {
                                let _ = event_tx.send(Event::TrayMenuItemClicked { id: selected });
                            }
                        }
                        let _ = event_tx.send(Event::TrayClicked {
                            button: MouseButton::Right,
                        });
                    }

                    WM_MBUTTONUP => {
                        let _ = event_tx.send(Event::TrayClicked {
                            button: MouseButton::Middle,
                        });
                    }

                    _ => {} // Ignore other notifications (WM_MOUSEMOVE, etc.)
                }
            }
        });
    } // unsafe
}

// --- Position change processing ---

fn process_position_changes(
    surfaces: &HashMap<SurfaceId, Surface>,
    event_tx: &mpsc::Sender<Event>,
    pending_saves: &mut HashMap<String, (i32, i32, Instant)>,
) {
    PENDING_POSITION_CHANGES.with(|q| {
        let events: Vec<_> = q.borrow_mut().drain(..).collect();
        for ev in events {
            // Find surface by HWND
            let Some((&id, surface)) = surfaces
                .iter()
                .find(|(_, s)| s.renderer.hwnd.0 as isize == ev.hwnd)
            else {
                continue;
            };
            // Emit event
            let _ = event_tx.send(Event::SurfaceMoved {
                surface_id: id,
                x: ev.x,
                y: ev.y,
            });
            // Update pending save (trailing-edge debounce)
            if let Some(ref key) = surface.position_key {
                pending_saves.insert(key.clone(), (ev.x, ev.y, Instant::now()));
            }
        }
    });
    // Flush entries older than 500ms
    let now = Instant::now();
    pending_saves.retain(|key, (x, y, ts)| {
        if now.duration_since(*ts) > Duration::from_millis(500) {
            persist::save_position(key, *x, *y);
            false // remove entry
        } else {
            true // keep pending
        }
    });
}

// --- Monitor event processing ---

#[cfg(target_os = "windows")]
unsafe fn process_monitor_events(
    surfaces: &mut HashMap<SurfaceId, Surface>,
    anchor_states: &mut HashMap<SurfaceId, AnchorState>,
    monitor: &mut WindowMonitor,
    event_tx: &mpsc::Sender<Event>,
) {
    // SAFETY: IsWindow check and window position APIs.
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::IsWindow;

        let events: Vec<MonitorEvent> =
            PENDING_MONITOR_EVENTS.with(|cell| cell.borrow_mut().drain(..).collect());

        for event in events {
            match event {
                MonitorEvent::LocationChanged { hwnd } => {
                    // Check if window still exists (detect close)
                    let target = HWND(hwnd as *mut _);
                    if !IsWindow(Some(target)).as_bool() {
                        handle_watched_window_closed(
                            hwnd,
                            surfaces,
                            anchor_states,
                            monitor,
                            event_tx,
                        );
                        continue;
                    }

                    // Reposition all anchored surfaces targeting this HWND
                    let surface_ids: Vec<SurfaceId> = anchor_states
                        .iter()
                        .filter(|(_, state)| state.target_hwnd == hwnd)
                        .map(|(id, _)| *id)
                        .collect();

                    for sid in surface_ids {
                        apply_anchor_position(surfaces, anchor_states, sid);
                    }
                }
                MonitorEvent::Minimized { hwnd } => {
                    if !IsWindow(Some(HWND(hwnd as *mut _))).as_bool() {
                        handle_watched_window_closed(
                            hwnd,
                            surfaces,
                            anchor_states,
                            monitor,
                            event_tx,
                        );
                        continue;
                    }

                    // Hide all anchored surfaces targeting this HWND
                    for (sid, state) in anchor_states.iter_mut() {
                        if state.target_hwnd == hwnd
                            && let Some(surface) = surfaces.get_mut(sid)
                        {
                            state.was_visible_before_minimize = surface.visible;
                            if surface.visible {
                                let _ = ShowWindow(surface.renderer.hwnd, SW_HIDE);
                                surface.visible = false;
                            }
                        }
                    }
                }
                MonitorEvent::Restored { hwnd } => {
                    // Show all previously-visible anchored surfaces targeting this HWND
                    for (sid, state) in anchor_states.iter_mut() {
                        if state.target_hwnd == hwnd && state.was_visible_before_minimize {
                            if let Some(surface) = surfaces.get_mut(sid) {
                                let _ = ShowWindow(surface.renderer.hwnd, SW_SHOWNOACTIVATE);
                                surface.visible = true;
                                surface.scene.set_dirty();
                            }
                            state.was_visible_before_minimize = false;
                        }
                    }

                    // Reposition
                    let surface_ids: Vec<SurfaceId> = anchor_states
                        .iter()
                        .filter(|(_, state)| state.target_hwnd == hwnd)
                        .map(|(id, _)| *id)
                        .collect();
                    for sid in surface_ids {
                        apply_anchor_position(surfaces, anchor_states, sid);
                    }
                }
            }
        }
    } // unsafe
}

#[cfg(not(target_os = "windows"))]
unsafe fn process_monitor_events(
    _surfaces: &mut HashMap<SurfaceId, Surface>,
    _anchor_states: &mut HashMap<SurfaceId, AnchorState>,
    _monitor: &mut WindowMonitor,
    _event_tx: &mpsc::Sender<Event>,
) {
}

// --- Watched window close handling ---

#[cfg(target_os = "windows")]
unsafe fn handle_watched_window_closed(
    hwnd: isize,
    _surfaces: &mut HashMap<SurfaceId, Surface>,
    anchor_states: &mut HashMap<SurfaceId, AnchorState>,
    monitor: &mut WindowMonitor,
    event_tx: &mpsc::Sender<Event>,
) {
    // Get all watchers for this HWND before removing
    let watches: Vec<Watch> = monitor
        .get_watches(hwnd)
        .map(<[Watch]>::to_vec)
        .unwrap_or_default();

    for watch in &watches {
        match &watch.reason {
            WatchReason::PipSource => {
                let _ = event_tx.send(Event::PipSourceClosed {
                    surface_id: watch.surface,
                });
                // Don't destroy the surface - consumer decides
            }
            WatchReason::AnchorTarget { .. } => {
                let _ = event_tx.send(Event::AnchorTargetClosed {
                    surface_id: watch.surface,
                });
                // Remove anchor state, surface stays at last position
                anchor_states.remove(&watch.surface);
            }
        }
        monitor.unwatch(hwnd, watch.surface);
    }
}

// --- PiP thumbnail property update ---

#[cfg(target_os = "windows")]
unsafe fn update_pip_thumbnail_properties(hwnd: HWND, pip: &PipState, renderer: &SurfaceRenderer) {
    // SAFETY: DWM thumbnail API with valid thumbnail handle.
    unsafe {
        let dpi = get_dpi_scale(hwnd);
        let phys_w = (renderer.width as f32 * dpi) as i32;
        let phys_h = (renderer.height as f32 * dpi) as i32;

        let mut props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE | DWM_TNP_OPACITY,
            rcDestination: RECT {
                left: 0,
                top: 0,
                right: phys_w,
                bottom: phys_h,
            },
            fVisible: TRUE,
            opacity: (pip.opacity * 255.0) as u8,
            ..Default::default()
        };

        if let Some(ref region) = pip.source_region {
            props.dwFlags |= DWM_TNP_RECTSOURCE;
            props.rcSource = RECT {
                left: region.x,
                top: region.y,
                right: region.x + region.width,
                bottom: region.y + region.height,
            };
        }

        let _ = DwmUpdateThumbnailProperties(pip.thumbnail, std::ptr::from_ref(&props));
    } // unsafe
}

#[cfg(not(target_os = "windows"))]
unsafe fn update_pip_thumbnail_properties(
    _hwnd: HWND,
    _pip: &PipState,
    _renderer: &SurfaceRenderer,
) {
}

// --- Anchor position calculation ---

#[cfg(target_os = "windows")]
unsafe fn apply_anchor_position(
    surfaces: &HashMap<SurfaceId, Surface>,
    anchor_states: &HashMap<SurfaceId, AnchorState>,
    surface_id: SurfaceId,
) {
    // SAFETY: GetWindowRect and SetWindowPos with valid HWNDs.
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

        let Some(state) = anchor_states.get(&surface_id) else {
            return;
        };
        let Some(surface) = surfaces.get(&surface_id) else {
            return;
        };

        let target_hwnd = HWND(state.target_hwnd as *mut _);
        let mut target_rect = RECT::default();
        if GetWindowRect(target_hwnd, &mut target_rect).is_err() {
            return;
        }

        let (base_x, base_y) = match state.anchor {
            Anchor::TopLeft => (target_rect.left, target_rect.top),
            Anchor::TopRight => (target_rect.right, target_rect.top),
            Anchor::BottomLeft => (target_rect.left, target_rect.bottom),
            Anchor::BottomRight => (target_rect.right, target_rect.bottom),
        };

        // Offset is stored in logical pixels (from the public API); GetWindowRect
        // returns physical screen coordinates. Scale the offset to physical pixels
        // using the surface's current DPI scale so anchoring is correct on
        // non-100% DPI monitors and adapts automatically on DPI changes.
        let dpi_scale = surface.renderer.dpi_scale;
        let x = base_x + (state.offset.0 as f32 * dpi_scale) as i32;
        let y = base_y + (state.offset.1 as f32 * dpi_scale) as i32;

        let _ = SetWindowPos(
            surface.renderer.hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
    } // unsafe
}

#[cfg(not(target_os = "windows"))]
unsafe fn apply_anchor_position(
    _surfaces: &HashMap<SurfaceId, Surface>,
    _anchor_states: &HashMap<SurfaceId, AnchorState>,
    _surface_id: SurfaceId,
) {
}

// --- Surface creation ---

unsafe fn create_hud_surface(
    gpu: &GpuResources,
    surfaces: &mut HashMap<SurfaceId, Surface>,
    next_id: &mut SurfaceId,
    config: HudConfig,
) -> Result<SurfaceId, Error> {
    // SAFETY: Window and renderer creation with valid GPU resources.
    unsafe {
        let monitors = display::enumerate_monitors();
        let (mut x, mut y) =
            display::resolve_placement(&config.placement, config.width, config.height, &monitors);

        // Restore saved position if position_key is set
        let mut physical_coords = matches!(config.placement, Placement::Monitor { .. });
        if let Some(ref key) = config.position_key
            && let Some((saved_x, saved_y)) = persist::load_position(key)
            && persist::is_position_on_screen(saved_x, saved_y, &monitors)
        {
            x = saved_x;
            y = saved_y;
            physical_coords = true; // saved positions are physical coords
        }

        let hwnd = create_hud_window(x, y, config.width, config.height)?;

        let renderer = match SurfaceRenderer::new(gpu, hwnd, config.width, config.height) {
            Ok(r) => r,
            Err(e) => {
                let _ = DestroyWindow(hwnd);
                return Err(e);
            }
        };

        // Adjust window position/size for DPI (CreateWindowExW uses physical pixels under PMv2)
        let scale = renderer.dpi_scale;
        let final_x = if physical_coords {
            x
        } else {
            (x as f32 * scale) as i32
        };
        let final_y = if physical_coords {
            y
        } else {
            (y as f32 * scale) as i32
        };
        let _ = SetWindowPos(
            hwnd,
            None,
            final_x,
            final_y,
            (config.width as f32 * scale) as i32,
            (config.height as f32 * scale) as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let id = *next_id;
        next_id.0 += 1;

        surfaces.insert(
            id,
            Surface {
                renderer,
                scene: SceneGraph::new(),
                visible: false,
                kind: SurfaceKind::Hud,
                opacity: 1.0,
                fading_out: false,
                position_key: config.position_key,
            },
        );

        Ok(id)
    } // unsafe
}

unsafe fn create_panel_surface(
    gpu: &GpuResources,
    surfaces: &mut HashMap<SurfaceId, Surface>,
    next_id: &mut SurfaceId,
    config: PanelConfig,
    event_tx: &mpsc::Sender<Event>,
) -> Result<SurfaceId, Error> {
    // SAFETY: Panel window creation with valid GPU resources and event sender.
    unsafe {
        let monitors = display::enumerate_monitors();
        let (mut x, mut y) =
            display::resolve_placement(&config.placement, config.width, config.height, &monitors);

        // Restore saved position if position_key is set
        let mut physical_coords = matches!(config.placement, Placement::Monitor { .. });
        if let Some(ref key) = config.position_key
            && let Some((saved_x, saved_y)) = persist::load_position(key)
            && persist::is_position_on_screen(saved_x, saved_y, &monitors)
        {
            x = saved_x;
            y = saved_y;
            physical_coords = true;
        }

        // 1. Create panel window
        let hwnd = create_panel_window(x, y, config.width, config.height)?;

        // 2. Create renderer (same pipeline as HUD)
        let renderer = match SurfaceRenderer::new(gpu, hwnd, config.width, config.height) {
            Ok(r) => r,
            Err(e) => {
                let _ = DestroyWindow(hwnd);
                return Err(e);
            }
        };

        // 3. DPI-scaled SetWindowPos (CreateWindowExW uses physical pixels under PMv2)
        let scale = renderer.dpi_scale;
        let final_x = if physical_coords {
            x
        } else {
            (x as f32 * scale) as i32
        };
        let final_y = if physical_coords {
            y
        } else {
            (y as f32 * scale) as i32
        };
        let _ = SetWindowPos(
            hwnd,
            None,
            final_x,
            final_y,
            (config.width as f32 * scale) as i32,
            (config.height as f32 * scale) as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        // 4. Create PanelState and store pointer in GWLP_USERDATA
        let id = *next_id;
        let panel_state = Box::new(PanelState {
            hit_test_map: HitTestMap::new(),
            event_sender: event_tx.clone(),
            surface_id: id,
            hovered_key: RefCell::new(None),
            draggable: config.draggable,
            drag_height: config.drag_height as f32 * scale,
            logical_drag_height: config.drag_height as f32,
            tracking_mouse: Cell::new(false),
        });

        // Safety: Box provides stable heap address. Pointer remains valid as long
        // as the Box exists (owned by Surface in the HashMap). GWLP_USERDATA is
        // cleared before the Box is dropped (in DestroySurface / Shutdown).
        let state_ptr = std::ptr::from_ref(&*panel_state) as isize;
        let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr);

        next_id.0 += 1;
        surfaces.insert(
            id,
            Surface {
                renderer,
                scene: SceneGraph::new(),
                visible: false,
                kind: SurfaceKind::Panel(panel_state),
                opacity: 1.0,
                fading_out: false,
                position_key: config.position_key,
            },
        );

        Ok(id)
    } // unsafe
}

// --- PiP surface creation ---

#[cfg(target_os = "windows")]
unsafe fn create_pip_surface(
    config: &PipConfig,
    gpu: &GpuResources,
    surfaces: &mut HashMap<SurfaceId, Surface>,
    next_id: &mut SurfaceId,
    monitor: &mut WindowMonitor,
) -> Result<SurfaceId, Error> {
    // SAFETY: PiP window and DWM thumbnail creation.
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::IsWindow;

        // Validate source window exists
        let source_hwnd = HWND(config.source_hwnd as *mut _);
        if !IsWindow(Some(source_hwnd)).as_bool() {
            return Err(Error::WindowCreation("source window is not valid".into()));
        }

        // Resolve placement
        let monitors = display::enumerate_monitors();
        let (mut x, mut y) =
            display::resolve_placement(&config.placement, config.width, config.height, &monitors);

        // Restore saved position if position_key is set
        let mut physical_coords = matches!(config.placement, Placement::Monitor { .. });
        if let Some(ref key) = config.position_key
            && let Some((saved_x, saved_y)) = persist::load_position(key)
            && persist::is_position_on_screen(saved_x, saved_y, &monitors)
        {
            x = saved_x;
            y = saved_y;
            physical_coords = true;
        }

        // Create window (same as HUD - click-through, topmost)
        let hwnd = create_hud_window(x, y, config.width, config.height)?;

        // Create SurfaceRenderer (GPU resources - unused for PiP but keeps Surface struct uniform)
        let renderer = match SurfaceRenderer::new(gpu, hwnd, config.width, config.height) {
            Ok(r) => r,
            Err(e) => {
                let _ = DestroyWindow(hwnd);
                return Err(e);
            }
        };

        // Get DPI scale for initial positioning
        let dpi = get_dpi_scale(hwnd);

        // Apply DPI-scaled position
        let final_x = if physical_coords {
            x
        } else {
            (x as f32 * dpi) as i32
        };
        let final_y = if physical_coords {
            y
        } else {
            (y as f32 * dpi) as i32
        };
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            final_x,
            final_y,
            (config.width as f32 * dpi) as i32,
            (config.height as f32 * dpi) as i32,
            SWP_NOACTIVATE,
        );

        // Register DWM thumbnail
        let thumbnail = DwmRegisterThumbnail(hwnd, source_hwnd)
            .map_err(|e| Error::WindowCreation(format!("DwmRegisterThumbnail: {e}")))?;

        // Set initial thumbnail properties: fill the entire destination window, visible
        let phys_w = (config.width as f32 * dpi) as i32;
        let phys_h = (config.height as f32 * dpi) as i32;

        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE | DWM_TNP_OPACITY,
            rcDestination: RECT {
                left: 0,
                top: 0,
                right: phys_w,
                bottom: phys_h,
            },
            fVisible: TRUE,
            opacity: 255,
            ..Default::default()
        };
        DwmUpdateThumbnailProperties(thumbnail, std::ptr::from_ref(&props))
            .map_err(|e| Error::WindowCreation(format!("DwmUpdateThumbnailProperties: {e}")))?;

        let id = *next_id;
        next_id.0 += 1;

        let pip_state = PipState {
            thumbnail,
            source_hwnd: config.source_hwnd,
            source_region: None,
            opacity: 1.0,
        };

        surfaces.insert(
            id,
            Surface {
                renderer,
                scene: SceneGraph::new(),
                visible: false,
                kind: SurfaceKind::Pip(pip_state),
                opacity: 1.0,
                fading_out: false,
                position_key: config.position_key.clone(),
            },
        );

        // Register source window in monitor for close detection
        monitor.watch(config.source_hwnd, id, WatchReason::PipSource);

        Ok(id)
    } // unsafe
}

#[cfg(not(target_os = "windows"))]
unsafe fn create_pip_surface(
    _config: &PipConfig,
    _gpu: &GpuResources,
    _surfaces: &mut HashMap<SurfaceId, Surface>,
    _next_id: &mut SurfaceId,
    _monitor: &mut WindowMonitor,
) -> Result<SurfaceId, Error> {
    Err(Error::UnsupportedOperation("PiP requires Windows".into()))
}

// --- Tray creation ---

unsafe fn create_tray(
    trays: &mut HashMap<TrayId, TrayState>,
    next_id: &mut TrayId,
    control_hwnd: HWND,
    config: TrayConfig,
) -> Result<TrayId, Error> {
    // SAFETY: Tray icon creation with valid HWND and RGBA data.
    unsafe {
        let hicon =
            create_hicon_from_rgba(&config.icon_rgba, config.icon_width, config.icon_height)?;

        let icon_id = next_id.0 as u32;
        create_tray_icon(control_hwnd, icon_id, hicon, &config.tooltip)?;

        let id = *next_id;
        next_id.0 += 1;

        trays.insert(
            id,
            TrayState {
                hwnd: control_hwnd,
                icon_id,
                hicon,
                popup_surface: None,
                popup_visible: false,
                menu_items: Vec::new(),
            },
        );

        Ok(id)
    } // unsafe
}
