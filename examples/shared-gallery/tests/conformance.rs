use shared_gallery::{settings_view, Action, Settings};
use ui_kit_core::{layout, Event, Point, Rect, Theme, Value};
use ui_kit_immediate::{ImmediateHost, Input};
#[cfg(feature = "gpu")]
#[test]
fn formatted_runs_render_as_safe_dioxus_spans() {
    use dioxus::prelude::*;
    fn app() -> Element {
        let mut doc =
            ui_kit_core::RichTextEditor::new(ui_kit_core::RichText::plain("<b>literal</b>"));
        doc.select_all();
        doc.format(ui_kit_core::FormatAction::Bold);
        rsx! {ui_kit_dioxus::RichTextView{document:doc.document}}
    }
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("font-weight:700"));
    assert!(html.contains("&#60;b&#62;literal&#60;/b&#62;"), "{html}");
    assert!(!html.contains("<b>literal</b>"));
}
fn bounds() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: 720.0,
        height: 600.0,
    }
}
fn point(state: &Settings, id: &str) -> Point {
    let view = settings_view(state);
    let placed = layout(&view, bounds());
    let r = placed
        .iter()
        .find(|p| p.view.id.as_deref() == Some(id))
        .unwrap()
        .rect;
    Point::new(r.x + r.width * 0.5, r.y + r.height * 0.5)
}
#[test]
fn both_hosts_emit_the_same_actions() {
    let state = Settings::default();
    let view = settings_view(&state);
    let theme = Theme::default();
    let mut host = ImmediateHost::default();
    for (id, value) in [("save", Value::Activate), ("enabled", Value::Toggle(false))] {
        let pointer = Some(point(&state, id));
        host.frame(
            &view,
            &theme,
            Input {
                pointer,
                down: true,
                ..Default::default()
            },
            bounds(),
        )
        .unwrap();
        let frame = host
            .frame(
                &view,
                &theme,
                Input {
                    pointer,
                    down: false,
                    ..Default::default()
                },
                bounds(),
            )
            .unwrap();
        let expected = ui_kit_dioxus::dispatch_event(
            &view,
            &Event {
                id: id.into(),
                value,
            },
        )
        .unwrap();
        assert_eq!(frame.actions, vec![expected]);
    }
    let pointer = Some(point(&state, "volume"));
    let frame = host
        .frame(
            &view,
            &theme,
            Input {
                pointer,
                down: true,
                ..Default::default()
            },
            bounds(),
        )
        .unwrap();
    assert_eq!(frame.actions, vec![Action::Volume(0.5)]);
    host.frame(
        &view,
        &theme,
        Input {
            pointer,
            down: false,
            ..Default::default()
        },
        bounds(),
    )
    .unwrap();
    let frame = host
        .frame(
            &view,
            &theme,
            Input {
                step: 1,
                ..Default::default()
            },
            bounds(),
        )
        .unwrap();
    assert!(matches!(frame.actions.as_slice(),[Action::Volume(v)] if (*v-0.6).abs()<1e-12));
}
#[test]
fn text_editing_disabled_and_loading_are_consistent() {
    let state = Settings::default();
    let view = settings_view(&state);
    let mut host = ImmediateHost::default();
    let theme = Theme::default();
    for id in ["disabled", "loading"] {
        let pointer = Some(point(&state, id));
        assert!(host
            .frame(
                &view,
                &theme,
                Input {
                    pointer,
                    down: true,
                    ..Default::default()
                },
                bounds()
            )
            .unwrap()
            .actions
            .is_empty());
        assert!(host
            .frame(
                &view,
                &theme,
                Input {
                    pointer,
                    ..Default::default()
                },
                bounds()
            )
            .unwrap()
            .actions
            .is_empty());
    }
    let pointer = Some(point(&state, "name"));
    host.frame(
        &view,
        &theme,
        Input {
            pointer,
            down: true,
            ..Default::default()
        },
        bounds(),
    )
    .unwrap();
    let frame = host
        .frame(
            &view,
            &theme,
            Input {
                text: " 🌍".into(),
                ..Default::default()
            },
            bounds(),
        )
        .unwrap();
    let expected = ui_kit_dioxus::dispatch_event(
        &view,
        &Event {
            id: "name".into(),
            value: Value::Text(format!("{} 🌍", state.name)),
        },
    )
    .unwrap();
    assert_eq!(frame.actions, vec![expected]);
}

#[cfg(feature = "gpu")]
#[test]
fn dioxus_outputs_native_semantic_controls() {
    use dioxus::prelude::*;
    use ui_kit_dioxus::SharedView;
    let mut dom = VirtualDom::new(
        || rsx! {SharedView{view:settings_view(&Settings::default()),on_action:move |_|{}}},
    );
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("type=\"checkbox\""));
    assert!(html.contains("type=\"range\""));
    assert!(html.contains("aria-label=\"Workspace name\""));
    assert!(html.contains("aria-busy=\"true\""));
    assert!(html.contains("disabled"));
}
