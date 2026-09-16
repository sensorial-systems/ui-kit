use ui_kit_core::*;
use ui_kit_immediate::{ImmediateHost, Input};

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Dark(bool),
    Size(usize),
}

fn frame(host: &mut ImmediateHost, view: &View<Action>, x: f32, y: f32, down: bool) -> Vec<Action> {
    host.frame(
        view,
        &Theme::default(),
        Input {
            pointer: Some(Point::new(x, y)),
            down,
            ..Default::default()
        },
        Rect::new(0.0, 0.0, 300.0, 400.0),
    )
    .unwrap()
    .actions
}

#[test]
fn the_immediate_host_draws_and_dispatches_the_newer_kinds() {
    let view = column([
        toggle("dark", "Dark mode", false).on_toggle(Action::Dark),
        select("size", "Size", ["Small", "Medium", "Large"], 0).on_select(Action::Size),
        progress("Upload", 0.5),
        separator(),
    ])
    .gap(0.0);
    let mut host = ImmediateHost::default();
    let mut actions = vec![];
    for down in [false, true, false] {
        actions.extend(frame(&mut host, &view, 10.0, 20.0, down));
    }
    assert_eq!(actions, vec![Action::Dark(true)]);

    // Open the select (44..88), then choose the third option under it.
    for down in [true, false] {
        frame(&mut host, &view, 50.0, 60.0, down);
    }
    let mut actions = vec![];
    for down in [false, true, false] {
        actions.extend(frame(&mut host, &view, 50.0, 90.0 + 44.0 * 2.5, down));
    }
    assert_eq!(actions, vec![Action::Size(2)]);
}
