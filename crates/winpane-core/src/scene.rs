use indexmap::IndexMap;

use crate::types::{ImageElement, RectElement, TextElement};

#[derive(Debug, Clone)]
pub enum Element {
    Text(TextElement),
    Rect(RectElement),
    Image(ImageElement),
}

pub(crate) struct SceneGraph {
    elements: IndexMap<String, Element>,
    dirty: bool,
}

impl SceneGraph {
    pub fn new() -> Self {
        SceneGraph {
            elements: IndexMap::new(),
            dirty: false,
        }
    }

    /// Insert or update an element. If key exists, value is replaced in-place
    /// (preserving insertion order). If key is new, appended to the end.
    pub fn set(&mut self, key: String, element: Element) {
        self.elements.insert(key, element);
        self.dirty = true;
    }

    /// Remove an element by key. Returns true if it existed.
    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.elements.shift_remove(key).is_some();
        if removed {
            self.dirty = true;
        }
        removed
    }

    /// Iterate elements in insertion order (back-to-front render order).
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Element)> {
        self.elements.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Get an element by key.
    pub fn get(&self, key: &str) -> Option<&Element> {
        self.elements.get(key)
    }

    /// Check and clear the dirty flag. Returns true if the scene was dirty.
    pub fn take_dirty(&mut self) -> bool {
        let was_dirty = self.dirty;
        self.dirty = false;
        was_dirty
    }

    /// Check dirty without clearing.
    #[allow(dead_code)]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Force dirty flag (used by engine for Show command).
    pub fn set_dirty(&mut self) {
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Color;

    fn text(name: &str) -> Element {
        Element::Text(TextElement {
            text: name.to_string(),
            ..Default::default()
        })
    }

    fn rect() -> Element {
        Element::Rect(RectElement {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: Color::BLACK,
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            interactive: false,
        })
    }

    #[test]
    fn insert_and_iterate_preserves_order() {
        let mut sg = SceneGraph::new();
        sg.set("a".into(), text("first"));
        sg.set("b".into(), text("second"));
        sg.set("c".into(), text("third"));
        let keys: Vec<&str> = sg.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn update_preserves_order() {
        let mut sg = SceneGraph::new();
        sg.set("a".into(), text("first"));
        sg.set("b".into(), text("second"));
        sg.set("c".into(), text("third"));
        sg.set("b".into(), text("updated"));
        let keys: Vec<&str> = sg.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn remove_and_reinsert_moves_to_end() {
        let mut sg = SceneGraph::new();
        sg.set("a".into(), text("first"));
        sg.set("b".into(), text("second"));
        sg.set("c".into(), text("third"));
        sg.remove("a");
        sg.set("a".into(), text("reinserted"));
        let keys: Vec<&str> = sg.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["b", "c", "a"]);
    }

    #[test]
    fn dirty_flag_lifecycle() {
        let mut sg = SceneGraph::new();
        assert!(!sg.is_dirty());

        sg.set("x".into(), rect());
        assert!(sg.is_dirty());

        assert!(sg.take_dirty());
        assert!(!sg.is_dirty());

        sg.remove("x");
        assert!(sg.is_dirty());
    }

    #[test]
    fn remove_nonexistent_not_dirty() {
        let mut sg = SceneGraph::new();
        sg.set("x".into(), rect());
        sg.take_dirty();

        let removed = sg.remove("nonexistent");
        assert!(!removed);
        assert!(!sg.is_dirty());
    }
}
