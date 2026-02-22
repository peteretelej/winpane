use std::collections::HashMap;
use std::sync::mpsc;

use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::command::{Command, CommandReceiver, CommandSender};
use crate::input::{HitTestMap, PanelState};
use crate::renderer::{GpuResources, SurfaceRenderer};
use crate::scene::SceneGraph;
use crate::tray::{
    create_hicon_from_rgba, create_tray_icon, destroy_tray_icon, show_tray_context_menu,
    update_tray_icon, update_tray_tooltip, TrayState,
};
use crate::types::{
    Error, Event, HudConfig, MouseButton, PanelConfig, SurfaceId, TrayConfig, TrayId,
};
use crate::window::{
    create_control_window, create_hud_window, create_panel_window, ensure_classes_registered,
    try_set_dpi_awareness, DpiChangeEvent, SendHwnd, PENDING_DPI_CHANGES, PENDING_TRAY_EVENTS,
};

// --- SurfaceKind ---

pub(crate) enum SurfaceKind {
    Hud,
    Panel(Box<PanelState>),
}

// --- Surface (internal to engine) ---

pub(crate) struct Surface {
    pub renderer: SurfaceRenderer,
    pub scene: SceneGraph,
    pub visible: bool,
    pub kind: SurfaceKind,
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
            .spawn(move || unsafe {
                engine_thread_main(cmd_rx, event_tx, ready_tx);
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
        unsafe {
            let _ = PostMessageW(self.control_hwnd.0, WM_APP, WPARAM(0), LPARAM(0));
        }
    }
}

/// Wake the engine thread's message loop from any thread.
/// This is a free function so that `Hud` (which only holds a `SendHwnd`)
/// can wake the engine without needing the full `EngineHandle`.
pub fn wake_engine(control_hwnd: SendHwnd) {
    unsafe {
        // Safety: PostMessageW is thread-safe by Win32 specification.
        // SendHwnd wraps HWND to allow cross-thread use.
        let _ = PostMessageW(control_hwnd.0, WM_APP, WPARAM(0), LPARAM(0));
    }
}

// --- Engine thread main ---

unsafe fn engine_thread_main(
    cmd_rx: CommandReceiver,
    event_tx: mpsc::Sender<Event>,
    ready_tx: mpsc::Sender<Result<SendHwnd, Error>>,
) {
    // 1. STA for message loop thread
    if let Err(e) = CoInitializeEx(None, COINIT_APARTMENTTHREADED) {
        let _ = ready_tx.send(Err(Error::DeviceCreation(format!("CoInitializeEx: {e}"))));
        return;
    }

    // 2. DPI awareness
    try_set_dpi_awareness();

    // 3. Register window classes
    ensure_classes_registered();

    // 4. GPU resources
    let gpu = match GpuResources::new() {
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

        // Drain command queue (non-blocking)
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                Command::Shutdown => {
                    // Destroy all trays
                    for (_, state) in trays.drain() {
                        destroy_tray_icon(state.hwnd, state.icon_id, state.hicon);
                    }
                    // Destroy all surfaces (clear panel GWLP_USERDATA first)
                    for (_, surface) in surfaces.drain() {
                        if let SurfaceKind::Panel(_) = &surface.kind {
                            let _ = SetWindowLongPtrW(surface.renderer.hwnd, GWLP_USERDATA, 0);
                        }
                        let _ = DestroyWindow(surface.renderer.hwnd);
                    }
                    let _ = DestroyWindow(control_hwnd);
                    return;
                }
                Command::CreateHud { config, reply } => {
                    let result = create_hud_surface(&gpu, &mut surfaces, &mut next_id, config);
                    let _ = reply.send(result);
                }
                Command::CreatePanel { config, reply } => {
                    let result =
                        create_panel_surface(&gpu, &mut surfaces, &mut next_id, config, &event_tx);
                    let _ = reply.send(result);
                }
                Command::CreateTray { config, reply } => {
                    let result = create_tray(&mut trays, &mut next_tray_id, control_hwnd, config);
                    let _ = reply.send(result);
                }
                Command::SetElement {
                    surface,
                    key,
                    element,
                } => {
                    if let Some(s) = surfaces.get_mut(&surface) {
                        s.scene.set(key, element);
                        // Rebuild hit-test map for panels
                        if let SurfaceKind::Panel(ref mut state) = s.kind {
                            state.hit_test_map.rebuild(&s.scene, s.renderer.dpi_scale);
                        }
                    }
                }
                Command::RemoveElement { surface, key } => {
                    if let Some(s) = surfaces.get_mut(&surface) {
                        s.scene.remove(&key);
                        // Rebuild hit-test map for panels
                        if let SurfaceKind::Panel(ref mut state) = s.kind {
                            state.hit_test_map.rebuild(&s.scene, s.renderer.dpi_scale);
                        }
                    }
                }
                Command::Show(id) => {
                    if let Some(s) = surfaces.get_mut(&id) {
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
                        s.scene.set_dirty();
                    }
                }
                Command::SetOpacity { surface, opacity } => {
                    if let Some(s) = surfaces.get(&surface) {
                        let _ = s.renderer.set_opacity(opacity);
                    }
                }
                Command::DestroySurface(id) => {
                    if let Some(surface) = surfaces.remove(&id) {
                        // Clear GWLP_USERDATA before DestroyWindow to prevent stale pointer access
                        if let SurfaceKind::Panel(_) = &surface.kind {
                            let _ = SetWindowLongPtrW(surface.renderer.hwnd, GWLP_USERDATA, 0);
                        }
                        let _ = DestroyWindow(surface.renderer.hwnd);
                    }
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
                    if let Some(state) = trays.get_mut(&tray) {
                        if let Ok(new_icon) = create_hicon_from_rgba(&rgba, width, height) {
                            let old_icon = state.hicon;
                            state.hicon = new_icon;
                            let _ = update_tray_icon(state.hwnd, state.icon_id, new_icon);
                            let _ = DestroyIcon(old_icon);
                        }
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
            }
        }

        // Render dirty visible surfaces
        for surface in surfaces.values_mut() {
            if surface.visible && surface.scene.take_dirty() {
                let _ = surface.renderer.render(&surface.scene, &gpu);
            }
        }
    }
}

// --- DPI change processing ---

fn process_dpi_changes(gpu: &GpuResources, surfaces: &mut HashMap<SurfaceId, Surface>) {
    PENDING_DPI_CHANGES.with(|changes| {
        for event in changes.borrow_mut().drain(..) {
            for surface in surfaces.values_mut() {
                if surface.renderer.hwnd == event.hwnd {
                    let new_scale = event.new_dpi as f32 / 96.0;
                    surface.renderer.dpi_scale = new_scale;
                    let _ = unsafe {
                        surface.renderer.resize(
                            gpu,
                            surface.renderer.width,
                            surface.renderer.height,
                        )
                    };
                    surface.scene.set_dirty();

                    // Update panel state for new DPI
                    if let SurfaceKind::Panel(ref mut state) = surface.kind {
                        state.hit_test_map.rebuild(&surface.scene, new_scale);
                    }

                    break;
                }
            }
        }
    });
}

// --- Tray event processing ---

unsafe fn process_tray_events(
    event_tx: &mpsc::Sender<Event>,
    trays: &mut HashMap<TrayId, TrayState>,
    surfaces: &mut HashMap<SurfaceId, Surface>,
) {
    PENDING_TRAY_EVENTS.with(|events| {
        for notification in events.borrow_mut().drain(..) {
            // Find the tray (for single-tray V1, there's at most one)
            let Some((_, tray_state)) = trays.iter_mut().next() else {
                continue;
            };

            match notification.event {
                // Left-click: toggle popup + send event
                WM_LBUTTONUP => {
                    if let Some(surface_id) = tray_state.popup_surface {
                        if let Some(surface) = surfaces.get_mut(&surface_id) {
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
}

// --- Surface creation ---

unsafe fn create_hud_surface(
    gpu: &GpuResources,
    surfaces: &mut HashMap<SurfaceId, Surface>,
    next_id: &mut SurfaceId,
    config: HudConfig,
) -> Result<SurfaceId, Error> {
    let hwnd = create_hud_window(config.x, config.y, config.width, config.height)?;

    let renderer = match SurfaceRenderer::new(gpu, hwnd, config.width, config.height) {
        Ok(r) => r,
        Err(e) => {
            let _ = DestroyWindow(hwnd);
            return Err(e);
        }
    };

    // Adjust window position/size for DPI (CreateWindowExW uses physical pixels under PMv2)
    let scale = renderer.dpi_scale;
    let _ = SetWindowPos(
        hwnd,
        None,
        (config.x as f32 * scale) as i32,
        (config.y as f32 * scale) as i32,
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
        },
    );

    Ok(id)
}

unsafe fn create_panel_surface(
    gpu: &GpuResources,
    surfaces: &mut HashMap<SurfaceId, Surface>,
    next_id: &mut SurfaceId,
    config: PanelConfig,
    event_tx: &mpsc::Sender<Event>,
) -> Result<SurfaceId, Error> {
    // 1. Create panel window
    let hwnd = create_panel_window(config.x, config.y, config.width, config.height)?;

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
    let _ = SetWindowPos(
        hwnd,
        None,
        (config.x as f32 * scale) as i32,
        (config.y as f32 * scale) as i32,
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
        hovered_key: None,
        draggable: config.draggable,
        drag_height: config.drag_height as f32 * scale,
        tracking_mouse: false,
    });

    // Safety: Box provides stable heap address. Pointer remains valid as long
    // as the Box exists (owned by Surface in the HashMap). GWLP_USERDATA is
    // cleared before the Box is dropped (in DestroySurface / Shutdown).
    let state_ptr = &*panel_state as *const PanelState as isize;
    let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr);

    next_id.0 += 1;
    surfaces.insert(
        id,
        Surface {
            renderer,
            scene: SceneGraph::new(),
            visible: false,
            kind: SurfaceKind::Panel(panel_state),
        },
    );

    Ok(id)
}

// --- Tray creation ---

unsafe fn create_tray(
    trays: &mut HashMap<TrayId, TrayState>,
    next_id: &mut TrayId,
    control_hwnd: HWND,
    config: TrayConfig,
) -> Result<TrayId, Error> {
    let hicon = create_hicon_from_rgba(&config.icon_rgba, config.icon_width, config.icon_height)?;

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
}
