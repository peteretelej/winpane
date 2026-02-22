//! Compile-time verification that Panel, Tray, and Event APIs exist and type-check.

#[cfg(target_os = "windows")]
mod windows_tests {
    use winpane::*;

    #[test]
    fn panel_config_construction() {
        let _ = PanelConfig {
            x: 0,
            y: 0,
            width: 200,
            height: 100,
            draggable: true,
            drag_height: 30,
        };
    }

    #[test]
    fn tray_config_construction() {
        let _ = TrayConfig {
            icon_rgba: vec![0u8; 32 * 32 * 4],
            icon_width: 32,
            icon_height: 32,
            tooltip: "test".into(),
        };
    }

    #[test]
    fn menu_item_construction() {
        let _ = MenuItem {
            id: 1,
            label: "Test".into(),
            enabled: true,
        };
    }

    #[test]
    fn event_matching() {
        let event = Event::ElementClicked {
            surface_id: SurfaceId(1),
            key: "btn".into(),
        };
        match event {
            Event::ElementClicked { surface_id, key } => {
                assert_eq!(surface_id, SurfaceId(1));
                assert_eq!(key, "btn");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn mouse_button_variants() {
        let _ = MouseButton::Left;
        let _ = MouseButton::Right;
        let _ = MouseButton::Middle;
    }

    #[test]
    fn context_and_panel_creation() {
        let ctx = Context::new().expect("Context::new failed");
        let panel = ctx
            .create_panel(PanelConfig {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                draggable: false,
                drag_height: 0,
            })
            .expect("create_panel failed");

        let _id = panel.id();
        assert!(ctx.poll_event().is_none());

        drop(panel);
    }
}
