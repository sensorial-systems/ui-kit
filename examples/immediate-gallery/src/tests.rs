use super::*;
#[test]
fn rich_editor_formats_selection_and_saves_only_on_commit() {
    let mut g = Gallery::default();
    let commands = render(&mut g);
    g.scroll = commands.iter().find(|c| c.text == "Edit").unwrap().rect.y - 150.0;
    let original = g.rich.saved.clone();
    click(&mut g, "Edit");
    render(&mut g);
    g.frame(
        Input {
            select_all: true,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    click(&mut g, "B");
    assert!(g.rich.draft.document.runs.iter().all(|r| r.format.bold));
    assert_eq!(g.rich.saved, original);
    click(&mut g, "Cancel");
    assert_eq!(g.rich.saved, original);
    click(&mut g, "Edit");
    render(&mut g);
    g.frame(
        Input {
            select_all: true,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    g.frame(
        Input {
            text: "Updated 👩‍💻".into(),
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    click(&mut g, "Save");
    assert_eq!(g.rich.saved.text(), "Updated 👩‍💻");
}
#[test]
fn rgb_and_hsl_controls_update_the_same_color() {
    let mut g = Gallery::default();
    g.picker_anchor = r(200.0, 100.0, 380.0, 408.0);
    g.overlay = Some(Overlay::Color);
    click(&mut g, "RGB");
    let pointer = Some(Point::new(220.0 + 340.0 * 0.8, 118.0 + 68.0 + 12.0));
    g.frame(
        Input {
            pointer,
            down: true,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    g.frame(
        Input {
            pointer,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    assert_eq!(g.color, "#cc82f6");
    click(&mut g, "HSL");
    let pointer = Some(Point::new(220.0, 118.0 + 68.0 + 62.0 + 12.0));
    g.frame(
        Input {
            pointer,
            down: true,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    let color = g.color_editor.color();
    assert!((color[0] - color[1]).abs() < 0.001 && (color[1] - color[2]).abs() < 0.001);
}
#[test]
fn survey_branches_on_role_and_submits_only_visible_answers() {
    let mut g = Gallery::default();
    g.overlay = Some(Overlay::Survey(0));
    click(&mut g, "Continue");
    click(&mut g, "Continue");
    assert!(matches!(g.overlay, Some(Overlay::Survey(1))));
    click(&mut g, "Software Engineer / Developer");
    click(&mut g, "Continue");
    assert!(matches!(g.overlay, Some(Overlay::Survey(2))));
    click(&mut g, "Rust & Dioxus");
    click(&mut g, "Continue");
    click(&mut g, "Back");
    click(&mut g, "Back");
    click(&mut g, "UI/UX Designer");
    click(&mut g, "Continue");
    assert!(matches!(g.overlay, Some(Overlay::Survey(3))));
    click(&mut g, "Submit");
    let result = g.survey_result.unwrap();
    assert_eq!(result.role, "designer");
    assert!(result.tech_stack.is_empty());
}
#[test]
fn select_all_replaces_native_hex_input() {
    let mut g = Gallery::default();
    g.picker_anchor = r(200.0, 100.0, 380.0, 408.0);
    g.overlay = Some(Overlay::Color);
    click(&mut g, "#3b82f6");
    g.frame(
        Input {
            select_all: true,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    g.frame(
        Input {
            text: "#ff000080".into(),
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    assert_eq!(g.color, "#ff000080");
    assert!((g.color_editor.alpha - 128.0 / 255.0).abs() < 0.001);
}
#[test]
fn calendar_handles_leap_years_and_clamps_month_changes() {
    assert_eq!(pickers::days(2000, 2), 29);
    assert_eq!(pickers::days(1900, 2), 28);
    let mut c = pickers::Calendar::from_value("2024-01-31 23:59");
    c.change_month(1);
    assert_eq!(c.value().as_deref(), Some("2024-02-29 23:59"));
    c.change_month(10);
    assert_eq!(c.month, 12);
    c.change_month(1);
    assert_eq!((c.year, c.month), (2025, 1));
    c.hour = "24".into();
    assert!(c.value().is_none());
}
#[test]
fn calendar_cancel_preserves_value_and_apply_commits() {
    let mut g = Gallery::default();
    g.picker_anchor = r(200.0, 100.0, 380.0, 418.0);
    g.overlay = Some(Overlay::Calendar);
    click(&mut g, "›");
    assert_eq!(g.calendar.month, 8);
    click(&mut g, "Cancel");
    assert_eq!(g.date, "2026-07-16 18:00");
    g.calendar = pickers::Calendar::from_value(&g.date);
    g.overlay = Some(Overlay::Calendar);
    click(&mut g, "›");
    click(&mut g, "Apply");
    assert_eq!(g.date, "2026-08-16 18:00");
    assert!(g.overlay.is_none());
}
#[test]
fn color_values_roundtrip_alpha_and_reject_invalid_input() {
    for value in ["#3b82f6", "#10b98180", "#000000", "#ffffff"] {
        let mut editor = pickers::ColorEditor::new(value);
        editor.update_hex();
        assert_eq!(editor.hex, value);
    }
    assert_eq!(
        pickers::parse_hex("#f008").unwrap(),
        [1.0, 0.0, 0.0, 136.0 / 255.0]
    );
    for value in ["#gggggg", "#12345", "#😀😀😀", "not a color"] {
        assert!(pickers::parse_hex(value).is_none());
    }
}
#[test]
fn color_canvas_respects_scrolled_coordinates() {
    let mut g = Gallery::default();
    let commands = render(&mut g);
    let canvas=commands.iter().find(|c|matches!(&c.style.material,Material::Custom{name,..} if name=="gallery.colored-triangles")).unwrap().rect;
    g.scroll = canvas.y - 200.0;
    let pointer = Some(Point::new(canvas.x + canvas.width * 0.75, 270.0));
    g.frame(
        Input {
            pointer,
            down: true,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    assert!((g.inline_editor.saturation - 0.75).abs() < 0.01);
    assert!((g.inline_editor.value - 0.5).abs() < 0.01);
}
#[test]
fn hierarchy_edits_preserve_ids_and_delete_subtrees() {
    let mut tree = hierarchy::Hierarchy::default();
    tree.select(1);
    let added = tree.add();
    tree.draft = "Native Views".into();
    assert!(tree.rename());
    assert_eq!(
        tree.nodes.iter().find(|n| n.id == added).unwrap().label,
        "Native Views"
    );
    tree.select(1);
    assert!(tree.delete());
    assert!(!tree
        .nodes
        .iter()
        .any(|n| [1, 4, 5, 6, 7, 8, 9, added].contains(&n.id)));
    assert_eq!(tree.selected, 0);
    assert!(!tree.delete());
    assert_eq!(tree.positions().len(), tree.nodes.len());
    assert!(tree.add() > added);
}
#[test]
fn hierarchy_editor_adds_and_renames_through_input() {
    let mut g = Gallery::default();
    let commands = render(&mut g);
    g.scroll = commands
        .iter()
        .find(|c| c.text == "Hierarchy Graph (Mind Map / Org Structure)")
        .unwrap()
        .rect
        .y
        - 30.0;
    click(&mut g, "Editable");
    click(&mut g, "Add child");
    let id = g.tree.selected;
    click(&mut g, "New node");
    g.frame(
        Input {
            text: " test".into(),
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    click(&mut g, "Rename");
    assert_eq!(
        g.tree.nodes.iter().find(|n| n.id == id).unwrap().label,
        "New node test"
    );
    click(&mut g, "Delete");
    assert!(!g.tree.nodes.iter().any(|n| n.id == id));
}
fn render(g: &mut Gallery) -> Vec<DrawCommand> {
    g.frame(Input::default(), 1200.0, 850.0, 0.0)
}
fn click(g: &mut Gallery, text: &str) {
    let mut commands = render(g);
    if g.overlay.is_some() {
        commands.reverse();
    }
    let c = commands
        .iter()
        .find(|c| c.text == text && c.rect.y >= 0.0 && c.rect.y < 850.0)
        .unwrap_or_else(|| panic!("missing visible control {text}"));
    let pointer = Some(Point::new(
        c.rect.x + c.rect.width * 0.5,
        c.rect.y + c.rect.height * 0.5,
    ));
    g.frame(
        Input {
            pointer,
            down: true,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    g.frame(
        Input {
            pointer,
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
}
fn section(g: &mut Gallery, name: &str) {
    render(g);
    g.scroll = g.section_positions[name];
}
#[test]
fn gallery_sections_have_finite_geometry() {
    let mut g = Gallery::default();
    let commands = render(&mut g);
    assert_eq!(g.section_positions.len(), SECTIONS.len());
    for c in commands {
        assert!([c.rect.x, c.rect.y, c.rect.width, c.rect.height]
            .iter()
            .all(|v| v.is_finite()));
        assert!(c.rect.width >= 0.0 && c.rect.height >= 0.0);
    }
    g.scroll_by(1e6);
    assert_eq!(g.scroll, g.content_height - 850.0);
    g.scroll_by(-1e6);
    assert_eq!(g.scroll, 0.0);
}
#[test]
fn theme_menu_changes_native_state() {
    let mut g = Gallery::default();
    click(&mut g, "White");
    assert!(g.overlay.is_some());
    click(&mut g, "Black");
    assert!(g.dark);
    assert!(g.overlay.is_none());
}
#[test]
fn loading_button_and_timeout_are_independent() {
    let mut g = Gallery::default();
    section(&mut g, "1. Buttons");
    click(&mut g, "Click to Load");
    assert!(g.loading);
    click(&mut g, "Reset");
    assert!(!g.loading);
    click(&mut g, "Load (3s Timeout)");
    assert!(g.timeout.is_some());
    g.frame(Input::default(), 1200.0, 850.0, 3.1);
    assert!(g.timeout.is_none());
}
#[test]
fn text_input_and_checkbox_update_state() {
    let mut g = Gallery::default();
    section(&mut g, "2. Form Controls");
    click(&mut g, "Enter username...");
    g.frame(
        Input {
            text: "Ada".into(),
            ..Default::default()
        },
        1200.0,
        850.0,
        0.0,
    );
    assert_eq!(g.username, "Ada");
    click(&mut g, "Accept terms and conditions");
    assert!(g.accepted);
}
#[test]
fn modal_blocks_background_and_scroll() {
    let mut g = Gallery::default();
    render(&mut g);
    g.overlay = Some(Overlay::Modal);
    let scroll = g.scroll;
    g.scroll_by(400.0);
    assert_eq!(g.scroll, scroll);
    click(&mut g, "Cancel");
    assert!(g.overlay.is_none());
}
#[test]
fn charts_use_batched_triangles() {
    let mut g = Gallery::default();
    let commands = render(&mut g);
    let vectors: Vec<_> = commands.iter().filter(|c| c.kind == "triangles").collect();
    assert!(!vectors.is_empty());
    assert!(vectors.len() < 400);
    for c in vectors {
        let Material::Custom { parameters, .. } = &c.style.material else {
            panic!("missing geometry")
        };
        assert_eq!(parameters.len() % 6, 0);
    }
}

#[test]
fn rgb_drag_preserves_other_channels_across_many_frames() {
    for channel in 0..3 {
        let mut g = Gallery::default();
        g.picker_anchor = r(200.0, 100.0, 380.0, 408.0);
        g.overlay = Some(Overlay::Color);
        click(&mut g, "RGB");
        let original = [0x3b, 0x82, 0xf6];
        for value in 1..255 {
            g.frame(
                Input {
                    pointer: Some(Point::new(
                        220.0 + 340.0 * value as f32 / 255.0,
                        198.0 + channel as f32 * 62.0,
                    )),
                    down: true,
                    ..Default::default()
                },
                1200.0,
                850.0,
                0.0,
            );
            let rgb = g.color_editor.color();
            for i in 0..3 {
                let expected = if i == channel { value } else { original[i] };
                assert_eq!(
                    (rgb[i] * 255.0).round() as i32,
                    expected,
                    "channel {channel}, value {value}, color {}",
                    g.color
                );
            }
        }
    }
}

#[test]
fn scrollbar_drag_tracks_pointer_clamps_and_releases() {
    let mut g = Gallery::default();
    render(&mut g);
    let thumb = (850.0 * 850.0 / g.content_height).max(24.0);
    let input = |y, down| Input {
        pointer: Some(Point::new(1195.0, y)),
        down,
        ..Default::default()
    };
    g.frame(input(10.0, true), 1200.0, 850.0, 0.0);
    assert_eq!(g.scroll, 0.0);
    g.frame(input(200.0, true), 1200.0, 850.0, 0.0);
    let expected = 190.0 / (850.0 - thumb) * (g.content_height - 850.0);
    assert!((g.scroll - expected).abs() < 0.1);
    g.frame(input(2000.0, true), 1200.0, 850.0, 0.0);
    assert_eq!(g.scroll, g.content_height - 850.0);
    g.frame(input(2000.0, false), 1200.0, 850.0, 0.0);
    g.frame(input(20.0, false), 1200.0, 850.0, 0.0);
    assert_eq!(g.scroll, g.content_height - 850.0);
    g.overlay = Some(Overlay::Modal);
    g.frame(input(10.0, true), 1200.0, 850.0, 0.0);
    assert_eq!(g.scroll, g.content_height - 850.0);
}

#[test]
fn flow_pan_moves_geometry_in_the_current_frame() {
    let mut g = Gallery::default();
    section(&mut g, "9. Graph Components");
    let before = render(&mut g)
        .iter()
        .find(|c| c.text == "1. Checkout")
        .unwrap()
        .rect;
    let input = |x, y| Input {
        pointer: Some(Point::new(x, y)),
        down: true,
        ..Default::default()
    };
    g.frame(input(250.0, 470.0), 1200.0, 850.0, 0.0);
    let moved = g.frame(input(310.0, 490.0), 1200.0, 850.0, 0.0);
    let after = moved.iter().find(|c| c.text == "1. Checkout").unwrap().rect;
    assert!((after.x - before.x - 60.0).abs() < 0.1);
    assert!((after.y - before.y - 20.0).abs() < 0.1);
}
