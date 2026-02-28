use std::cell::{Cell, RefCell};
use std::sync::mpsc;

use crate::scene::{Element, SceneGraph};
use crate::types::{Event, SurfaceId};

// --- ElementBounds ---

pub(crate) struct ElementBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// --- Element methods ---

impl Element {
    pub(crate) fn is_interactive(&self) -> bool {
        match self {
            Element::Text(t) => t.interactive,
            Element::Rect(r) => r.interactive,
            Element::Image(i) => i.interactive,
        }
    }

    pub(crate) fn bounds(&self) -> ElementBounds {
        match self {
            Element::Rect(r) => ElementBounds {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
            },
            Element::Image(i) => ElementBounds {
                x: i.x,
                y: i.y,
                width: i.width,
                height: i.height,
            },
            Element::Text(t) => {
                let est_width = t.font_size * t.text.chars().count() as f32 * 0.6;
                let est_height = t.font_size * 1.3;
                ElementBounds {
                    x: t.x,
                    y: t.y,
                    width: est_width,
                    height: est_height,
                }
            }
        }
    }
}

// --- HitRegion ---

pub(crate) struct HitRegion {
    pub key: String,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

// --- HitTestMap ---

pub(crate) struct HitTestMap {
    regions: Vec<HitRegion>,
}

impl HitTestMap {
    pub fn new() -> Self {
        HitTestMap {
            regions: Vec::new(),
        }
    }

    pub fn rebuild(&mut self, scene: &SceneGraph, dpi_scale: f32) {
        self.regions.clear();
        for (key, element) in scene.iter() {
            if element.is_interactive() {
                let b = element.bounds();
                self.regions.push(HitRegion {
                    key: key.to_string(),
                    left: b.x * dpi_scale,
                    top: b.y * dpi_scale,
                    right: (b.x + b.width) * dpi_scale,
                    bottom: (b.y + b.height) * dpi_scale,
                });
            }
        }
    }

    /// Returns the key of the topmost interactive element at the given physical-pixel coordinates.
    /// Iterates back-to-front; last match wins (topmost in z-order).
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&str> {
        let mut result = None;
        for region in &self.regions {
            if x >= region.left && x < region.right && y >= region.top && y < region.bottom {
                result = Some(region.key.as_str());
            }
        }
        result
    }
}

// --- PanelState ---

pub(crate) struct PanelState {
    pub hit_test_map: HitTestMap,
    pub event_sender: mpsc::Sender<Event>,
    pub surface_id: SurfaceId,
    pub hovered_key: RefCell<Option<String>>,
    pub draggable: bool,
    /// Physical pixels from top of panel for drag handle.
    pub drag_height: f32,
    /// Logical pixels from top of panel for drag handle (DPI-independent).
    pub logical_drag_height: f32,
    /// Whether TrackMouseEvent(TME_LEAVE) is active.
    pub tracking_mouse: Cell<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn empty_map_returns_none() {
        let map = HitTestMap::new();
        assert!(map.hit_test(50.0, 50.0).is_none());
    }

    #[test]
    fn hit_test_finds_interactive_rect() {
        let mut scene = SceneGraph::new();
        scene.set(
            "btn".into(),
            Element::Rect(RectElement {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
                fill: Color::BLACK,
                corner_radius: 0.0,
                border_color: None,
                border_width: 0.0,
                interactive: true,
            }),
        );
        scene.set(
            "bg".into(),
            Element::Rect(RectElement {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
                fill: Color::BLACK,
                corner_radius: 0.0,
                border_color: None,
                border_width: 0.0,
                interactive: false,
            }),
        );
        let mut map = HitTestMap::new();
        map.rebuild(&scene, 1.0);

        // Inside interactive rect
        assert_eq!(map.hit_test(50.0, 30.0), Some("btn"));
        // Inside non-interactive rect only
        assert!(map.hit_test(150.0, 150.0).is_none());
        // Outside all rects
        assert!(map.hit_test(250.0, 250.0).is_none());
    }

    #[test]
    fn hit_test_topmost_wins() {
        let mut scene = SceneGraph::new();
        // "bottom" inserted first (backmost)
        scene.set(
            "bottom".into(),
            Element::Rect(RectElement {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
                fill: Color::BLACK,
                corner_radius: 0.0,
                border_color: None,
                border_width: 0.0,
                interactive: true,
            }),
        );
        // "top" inserted second (frontmost)
        scene.set(
            "top".into(),
            Element::Rect(RectElement {
                x: 20.0,
                y: 20.0,
                width: 60.0,
                height: 60.0,
                fill: Color::BLACK,
                corner_radius: 0.0,
                border_color: None,
                border_width: 0.0,
                interactive: true,
            }),
        );
        let mut map = HitTestMap::new();
        map.rebuild(&scene, 1.0);

        // Overlap region: topmost wins
        assert_eq!(map.hit_test(50.0, 50.0), Some("top"));
        // Only bottom
        assert_eq!(map.hit_test(5.0, 5.0), Some("bottom"));
    }

    #[test]
    fn hit_test_respects_dpi_scale() {
        let mut scene = SceneGraph::new();
        scene.set(
            "btn".into(),
            Element::Rect(RectElement {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
                fill: Color::BLACK,
                corner_radius: 0.0,
                border_color: None,
                border_width: 0.0,
                interactive: true,
            }),
        );
        let mut map = HitTestMap::new();
        map.rebuild(&scene, 1.5); // 150% DPI

        // Physical coords: rect is at (15, 15) to (165, 90)
        assert_eq!(map.hit_test(20.0, 20.0), Some("btn"));
        assert!(map.hit_test(10.0, 10.0).is_none()); // below physical left/top
    }

    #[test]
    fn non_interactive_elements_skipped() {
        let mut scene = SceneGraph::new();
        scene.set(
            "a".into(),
            Element::Rect(RectElement {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
                fill: Color::BLACK,
                corner_radius: 0.0,
                border_color: None,
                border_width: 0.0,
                interactive: false,
            }),
        );
        let mut map = HitTestMap::new();
        map.rebuild(&scene, 1.0);

        assert!(map.hit_test(50.0, 50.0).is_none());
        assert!(map.regions.is_empty());
    }
}
