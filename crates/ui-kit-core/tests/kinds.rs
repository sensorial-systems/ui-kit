use ui_kit_core::*;

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Dark(bool),
    Size(usize),
}

#[test]
fn toggles_and_selects_dispatch_and_validate() {
    let view = column([
        toggle("dark", "Dark mode", false).on_toggle(Action::Dark),
        select("size", "Size", ["Small", "Medium", "Large"], 0).on_select(Action::Size),
        progress("Upload", 0.5),
        separator(),
    ]);
    view.validate().unwrap();
    assert_eq!(
        view.dispatch(&Event {
            id: "dark".into(),
            value: Value::Toggle(true)
        }),
        Some(Action::Dark(true))
    );
    assert_eq!(
        view.dispatch(&Event {
            id: "size".into(),
            value: Value::Select(2)
        }),
        Some(Action::Size(2))
    );
    assert_eq!(
        view.dispatch(&Event {
            id: "size".into(),
            value: Value::Select(3)
        }),
        None,
        "an index past the options is ignored"
    );
    assert!(select::<Action>("bad", "Bad", ["one"], 4)
        .validate()
        .is_err());
    assert!(progress::<Action>("bad", 1.5).validate().is_err());
}
