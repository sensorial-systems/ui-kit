use ui_kit_core::*;
#[derive(Clone, Debug, PartialEq)]
enum Action {
    Save,
    Value(f64),
}
#[test]
fn dispatch_validates_kind_disabled_loading_and_range() {
    let view = column([
        button("save", "Save").on_activate(Action::Save),
        slider("value", "Value", 0.0)
            .range(0.0, 10.0, 0.5)
            .on_change(Action::Value),
    ]);
    assert_eq!(
        view.dispatch(&Event {
            id: "value".into(),
            value: Value::Number(3.24)
        }),
        Some(Action::Value(3.0))
    );
    assert_eq!(
        view.dispatch(&Event {
            id: "value".into(),
            value: Value::Number(f64::NAN)
        }),
        None
    );
    assert_eq!(
        view.dispatch(&Event {
            id: "save".into(),
            value: Value::Number(1.0)
        }),
        None
    );
    assert_eq!(
        view.clone().disabled(true).dispatch(&Event {
            id: "save".into(),
            value: Value::Activate
        }),
        None
    );
    assert_eq!(
        button("save", "Save")
            .loading(true)
            .on_activate(Action::Save)
            .dispatch(&Event {
                id: "save".into(),
                value: Value::Activate
            }),
        None
    );
}
#[test]
fn invalid_tree_is_rejected_and_layout_is_composable() {
    assert!(column([button::<()>("same", "A"), button("same", "B")])
        .validate()
        .is_err());
    assert!(slider::<()>("s", "", 0.0)
        .range(1.0, 0.0, 1.0)
        .validate()
        .is_err());
    let tree = row([button::<()>("a", "A").width(80.0), button("b", "B")])
        .padding(10.0)
        .gap(20.0);
    let placed = layout(
        &tree,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 64.0,
        },
    );
    assert_eq!(
        placed[1].rect,
        Rect {
            x: 10.0,
            y: 10.0,
            width: 80.0,
            height: 44.0
        }
    );
    assert_eq!(
        placed[2].rect,
        Rect {
            x: 110.0,
            y: 10.0,
            width: 180.0,
            height: 44.0
        }
    );
}
