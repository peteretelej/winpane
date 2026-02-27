//! Window monitor: observes external windows for position changes,
//! minimize/restore, and destruction. Used by PiP and anchoring.

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::HWND,
    UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
    UI::WindowsAndMessaging::{
        EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
        WINEVENT_OUTOFCONTEXT,
    },
};

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::types::{Anchor, SurfaceId};

// --- Monitor event queue (thread-local, same thread as message loop) ---

#[derive(Debug)]
pub(crate) enum MonitorEvent {
    LocationChanged { hwnd: isize },
    Minimized { hwnd: isize },
    Restored { hwnd: isize },
}

thread_local! {
    pub(crate) static PENDING_MONITOR_EVENTS: RefCell<Vec<MonitorEvent>> =
        const { RefCell::new(Vec::new()) };
    static WATCHED_HWNDS: RefCell<HashSet<isize>> =
        RefCell::new(HashSet::new());
}

// --- Watch tracking ---

#[derive(Debug, Clone)]
pub(crate) enum WatchReason {
    PipSource,
    #[allow(dead_code)]
    AnchorTarget {
        anchor: Anchor,
        offset: (i32, i32),
    },
}

#[derive(Debug, Clone)]
pub(crate) struct Watch {
    pub surface: SurfaceId,
    pub reason: WatchReason,
}

// --- WindowMonitor ---

pub(crate) struct WindowMonitor {
    /// HWND (as isize) -> list of watchers
    watched: HashMap<isize, Vec<Watch>>,
    /// Active SetWinEventHook handles
    #[cfg(target_os = "windows")]
    hooks: Vec<HWINEVENTHOOK>,
    #[cfg(not(target_os = "windows"))]
    hooks: Vec<()>,
}

impl WindowMonitor {
    pub fn new() -> Self {
        WindowMonitor {
            watched: HashMap::new(),
            hooks: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.watched.is_empty()
    }

    /// Start watching a window for a given surface/reason.
    /// Registers SetWinEventHook lazily on first watch.
    pub fn watch(&mut self, hwnd: isize, surface: SurfaceId, reason: WatchReason) {
        let was_empty = self.watched.is_empty();
        self.watched
            .entry(hwnd)
            .or_default()
            .push(Watch { surface, reason });
        WATCHED_HWNDS.with(|set| {
            set.borrow_mut().insert(hwnd);
        });
        if was_empty {
            self.register_hooks();
        }
    }

    /// Stop watching for a specific surface (all reasons).
    /// Unregisters hooks when the last watcher is removed.
    pub fn unwatch_surface(&mut self, surface: SurfaceId) {
        self.watched.retain(|_, watches| {
            watches.retain(|w| w.surface != surface);
            !watches.is_empty()
        });
        WATCHED_HWNDS.with(|set| {
            let mut s = set.borrow_mut();
            s.clear();
            s.extend(self.watched.keys());
        });
        if self.watched.is_empty() {
            self.unregister_hooks();
        }
    }

    /// Stop watching a specific HWND for a specific surface.
    pub fn unwatch(&mut self, hwnd: isize, surface: SurfaceId) {
        if let Some(watches) = self.watched.get_mut(&hwnd) {
            watches.retain(|w| w.surface != surface);
            if watches.is_empty() {
                self.watched.remove(&hwnd);
            }
        }
        if !self.watched.contains_key(&hwnd) {
            WATCHED_HWNDS.with(|set| {
                set.borrow_mut().remove(&hwnd);
            });
        }
        if self.watched.is_empty() {
            self.unregister_hooks();
        }
    }

    /// Returns all watchers for a given HWND.
    pub fn get_watches(&self, hwnd: isize) -> Option<&[Watch]> {
        self.watched.get(&hwnd).map(Vec::as_slice)
    }

    /// Returns all watched HWNDs.
    #[allow(dead_code)]
    pub fn watched_hwnds(&self) -> impl Iterator<Item = &isize> {
        self.watched.keys()
    }
}

// --- Hook registration (Windows-only) ---

#[cfg(target_os = "windows")]
impl WindowMonitor {
    fn register_hooks(&mut self) {
        if !self.hooks.is_empty() {
            return; // already registered
        }
        // SAFETY: SetWinEventHook with valid callback; hooks stored for later cleanup.
        unsafe {
            // Hook 1: location changes (move/resize)
            let h1 = SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                None,
                Some(monitor_event_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if !h1.is_invalid() {
                self.hooks.push(h1);
            }

            // Hook 2: minimize/restore
            let h2 = SetWinEventHook(
                EVENT_SYSTEM_MINIMIZESTART,
                EVENT_SYSTEM_MINIMIZEEND,
                None,
                Some(monitor_event_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if !h2.is_invalid() {
                self.hooks.push(h2);
            }
        }
    }

    fn unregister_hooks(&mut self) {
        for hook in self.hooks.drain(..) {
            // SAFETY: UnhookWinEvent with handles obtained from SetWinEventHook.
            unsafe {
                let _ = UnhookWinEvent(hook);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl WindowMonitor {
    fn register_hooks(&mut self) {}
    fn unregister_hooks(&mut self) {}
}

// --- WinEvent callback ---

#[cfg(target_os = "windows")]
/// Safety: Called by SetWinEventHook. Parameters come directly from Windows.
/// We validate id_object == 0 (window-level) and check WATCHED_HWNDS before processing.
#[allow(dead_code)]
unsafe extern "system" fn monitor_event_callback(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    // Only care about window-level events (OBJID_WINDOW = 0)
    if id_object != 0 {
        return;
    }

    let hwnd_val = hwnd.0 as isize;
    let is_watched = WATCHED_HWNDS.with(|set| set.borrow().contains(&hwnd_val));
    if !is_watched {
        return;
    }

    let monitor_event = match event {
        e if e == EVENT_OBJECT_LOCATIONCHANGE => MonitorEvent::LocationChanged { hwnd: hwnd_val },
        e if e == EVENT_SYSTEM_MINIMIZESTART => MonitorEvent::Minimized { hwnd: hwnd_val },
        e if e == EVENT_SYSTEM_MINIMIZEEND => MonitorEvent::Restored { hwnd: hwnd_val },
        _ => return,
    };

    PENDING_MONITOR_EVENTS.with(|cell| {
        cell.borrow_mut().push(monitor_event);
    });
}

// --- Cleanup ---

impl Drop for WindowMonitor {
    fn drop(&mut self) {
        self.unregister_hooks();
    }
}
