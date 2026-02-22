use std::collections::HashMap;
use std::sync::mpsc;

use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::command::{Command, CommandReceiver, CommandSender};
use crate::renderer::{GpuResources, SurfaceRenderer};
use crate::scene::SceneGraph;
use crate::types::{Error, HudConfig, SurfaceId};
use crate::window::{
    create_control_window, create_hud_window, ensure_classes_registered, try_set_dpi_awareness,
    DpiChangeEvent, SendHwnd, PENDING_DPI_CHANGES,
};

// --- Surface (internal to engine) ---

pub(crate) struct Surface {
    pub renderer: SurfaceRenderer,
    pub scene: SceneGraph,
    pub visible: bool,
}

// --- EngineHandle (returned to winpane crate) ---

pub struct EngineHandle {
    pub sender: CommandSender,
    pub control_hwnd: SendHwnd,
    pub join_handle: Option<std::thread::JoinHandle<()>>,
}

impl EngineHandle {
    pub fn spawn() -> Result<Self, Error> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<SendHwnd, Error>>();

        let handle = std::thread::Builder::new()
            .name("winpane-engine".into())
            .spawn(move || unsafe {
                engine_thread_main(cmd_rx, ready_tx);
            })
            .map_err(|_| Error::ThreadSpawnFailed)?;

        // Block until the engine thread signals ready
        let control_hwnd = ready_rx.recv().map_err(|_| Error::ThreadSpawnFailed)??;

        Ok(EngineHandle {
            sender: cmd_tx,
            control_hwnd,
            join_handle: Some(handle),
        })
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

    // 8. Message loop
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

        // Drain command queue (non-blocking)
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                Command::Shutdown => {
                    for (_, surface) in surfaces.drain() {
                        let _ = DestroyWindow(surface.renderer.hwnd);
                    }
                    let _ = DestroyWindow(control_hwnd);
                    return;
                }
                Command::CreateHud { config, reply } => {
                    let result = create_hud_surface(&gpu, &mut surfaces, &mut next_id, config);
                    let _ = reply.send(result);
                }
                Command::SetElement {
                    surface,
                    key,
                    element,
                } => {
                    if let Some(s) = surfaces.get_mut(&surface) {
                        s.scene.set(key, element);
                    }
                }
                Command::RemoveElement { surface, key } => {
                    if let Some(s) = surfaces.get_mut(&surface) {
                        s.scene.remove(&key);
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
                        let _ = DestroyWindow(surface.renderer.hwnd);
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
                    break;
                }
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
        },
    );

    Ok(id)
}
