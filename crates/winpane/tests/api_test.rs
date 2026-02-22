//! Integration test: validates the winpane public API surface compiles
//! and types can be constructed.
//!
//! Actual rendering requires a Windows GPU environment, so this test
//! focuses on type construction and API ergonomics.

#[cfg(target_os = "windows")]
mod windows_tests {
    use winpane::{Color, Error, HudConfig, ImageElement, RectElement, TextElement};

    #[test]
    fn color_constants_exist() {
        let _ = Color::WHITE;
        let _ = Color::BLACK;
        let _ = Color::TRANSPARENT;
    }

    #[test]
    fn color_constructors() {
        let c = Color::rgba(10, 20, 30, 40);
        assert_eq!(c.r, 10);
        assert_eq!(c.g, 20);
        assert_eq!(c.b, 30);
        assert_eq!(c.a, 40);

        let c2 = Color::rgb(100, 200, 50);
        assert_eq!(c2.a, 255);
    }

    #[test]
    fn hud_config_construction() {
        let config = HudConfig {
            x: 100,
            y: 200,
            width: 320,
            height: 180,
        };
        assert_eq!(config.x, 100);
        assert_eq!(config.width, 320);
    }

    #[test]
    fn text_element_default() {
        let t = TextElement::default();
        assert!(t.text.is_empty());
        assert_eq!(t.font_size, 14.0);
        assert_eq!(t.color, Color::WHITE);
        assert!(!t.bold);
        assert!(!t.italic);
        assert!(t.font_family.is_none());
    }

    #[test]
    fn rect_element_construction() {
        let r = RectElement {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
            fill: Color::BLACK,
            corner_radius: 5.0,
            border_color: Some(Color::WHITE),
            border_width: 1.0,
        };
        assert_eq!(r.width, 100.0);
        assert!(r.border_color.is_some());
    }

    #[test]
    fn image_element_construction() {
        let img = ImageElement {
            x: 10.0,
            y: 20.0,
            width: 64.0,
            height: 64.0,
            data: vec![0u8; 64 * 64 * 4],
            data_width: 64,
            data_height: 64,
        };
        assert_eq!(img.data.len(), 64 * 64 * 4);
    }

    #[test]
    fn context_new_returns_result() {
        // Verify the return type compiles. Actually calling Context::new()
        // spawns the engine thread, which we test here to validate the
        // full pipeline on Windows CI.
        let result = winpane::Context::new();
        assert!(result.is_ok(), "Context::new() failed: {:?}", result.err());
        // Drop the context, which shuts down the engine thread
    }

    #[test]
    fn error_display() {
        let e = Error::Shutdown;
        let msg = format!("{e}");
        assert!(!msg.is_empty());
    }
}
