use ui_kit_core::Point as Vec2;
use ui_kit_immediate::immediate::*;
use ui_kit_immediate::spatial::Vec3;
fn rect() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 40.0,
    }
}
fn input(x: f32, down: bool) -> Input {
    Input {
        pointer: Some(Vec2::new(x, 20.0)),
        down,
        ..Default::default()
    }
}
#[test]
fn capture_requires_press_inside_and_release_inside() {
    let mut ui = Ui::default();
    for (x, down, clicked) in [
        (120.0, true, false),
        (20.0, false, false),
        (20.0, true, false),
        (120.0, false, false),
        (20.0, true, false),
        (20.0, false, true),
        (20.0, false, false),
    ] {
        ui.begin(input(x, down));
        assert_eq!(ui.button("b", rect(), "Button").clicked, clicked);
        ui.end();
    }
}
#[test]
fn clipped_controls_do_not_capture_and_disappearing_controls_release() {
    let mut ui = Ui::default();
    ui.clip = Some(Rect { x: 50.0, ..rect() });
    ui.begin(input(20.0, true));
    assert!(!ui.button("b", rect(), "").active);
    ui.end();
    ui.begin(input(20.0, false));
    ui.end();
    ui.clip = None;
    ui.begin(input(20.0, true));
    assert!(ui.button("b", rect(), "").active);
    ui.end();
    ui.begin(input(20.0, true));
    ui.end();
    ui.begin(input(20.0, false));
    assert!(!ui.button("b", rect(), "").clicked);
    ui.end();
}
#[test]
fn unicode_edit_and_keyboard_focus() {
    let mut ui = Ui::default();
    let mut value = "A🌍".to_string();
    ui.begin(Input::default());
    ui.text_input("text", rect(), &mut value);
    ui.end();
    ui.begin(Input {
        tab: true,
        backspace: true,
        text: "é".into(),
        ..Default::default()
    });
    assert!(ui.text_input("text", rect(), &mut value).changed);
    ui.end();
    assert_eq!(value, "Aé");
}
#[test]
fn world_panel_round_trip_and_misses() {
    let panel = WorldPanel {
        origin: Vec3::new(0.0, 0.0, -2.0),
        right: Vec3::new(2.0, 0.0, 0.0),
        down: Vec3::new(0.5, -1.0, 0.0),
        pixels: Vec2::new(800.0, 400.0),
    };
    let p = Vec2::new(200.0, 300.0);
    let world = panel.point(p);
    let hit = panel.hit(Vec3::default(), world).unwrap();
    assert!((hit.x - p.x).abs() < 0.001 && (hit.y - p.y).abs() < 0.001);
    assert!(panel
        .hit(Vec3::default(), Vec3::new(0.0, 0.0, 1.0))
        .is_none());
    assert!(panel
        .hit(Vec3::default(), Vec3::new(1.0, 0.0, 0.0))
        .is_none());
    assert!(panel
        .hit(Vec3::default(), Vec3::new(-5.0, 0.0, -2.0))
        .is_none());
}
