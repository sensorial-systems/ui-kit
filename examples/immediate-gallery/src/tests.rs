use super::*;
use ui_kit_core::Material;

const W: f32 = 1200.0;
const H: f32 = 850.0;

fn render(g: &mut Gallery) -> Vec<DrawCommand> {
    g.frame(Input::default(), W, H, 0.0)
}

fn frame_at(g: &mut Gallery, x: f32, y: f32, down: bool) -> Vec<DrawCommand> {
    g.frame(
        Input {
            pointer: Some(Point::new(x, y)),
            down,
            ..Default::default()
        },
        W,
        H,
        0.0,
    )
}

/// The topmost command showing `text`, scrolled into view first.
fn find(g: &mut Gallery, text: &str) -> Rect {
    let visible = |commands: &[DrawCommand]| {
        commands
            .iter()
            .rev()
            .find(|c| c.text == text && c.rect.y >= 0.0 && c.rect.bottom() <= H)
            .map(|c| c.rect)
    };
    let commands = render(g);
    if let Some(rect) = visible(&commands) {
        return rect;
    }
    let rect = commands
        .iter()
        .rev()
        .find(|c| c.text == text)
        .unwrap_or_else(|| panic!("missing control {text}"))
        .rect;
    g.scroll_by(rect.y - 300.0);
    visible(&render(g)).unwrap_or_else(|| panic!("control {text} not visible"))
}

fn press_release(g: &mut Gallery, rect: Rect) {
    let (x, y) = (rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
    frame_at(g, x, y, false);
    frame_at(g, x, y, true);
    frame_at(g, x, y, false);
}

fn click(g: &mut Gallery, text: &str) {
    let rect = find(g, text);
    press_release(g, rect);
}

/// The first command showing `text`, as drawn on the page before any popup.
fn first(g: &mut Gallery, text: &str) -> Rect {
    render(g).iter().find(|c| c.text == text).unwrap().rect
}

fn section(g: &mut Gallery, name: &str) {
    render(g);
    g.scroll = g.section_positions[name];
}

fn type_text(g: &mut Gallery, text: &str) {
    g.frame(
        Input {
            text: text.into(),
            ..Default::default()
        },
        W,
        H,
        0.0,
    );
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
    assert_eq!(g.scroll, g.content_height - H);
    g.scroll_by(-1e6);
    assert_eq!(g.scroll, 0.0);
}

#[test]
fn text_is_plain_ascii_and_symbols_are_vector_icons() {
    let mut g = Gallery::default();
    g.table.loading = true;
    let mut texts: Vec<String> = render(&mut g).into_iter().map(|c| c.text).collect();
    let products = first(&mut g, "Products");
    press_release(&mut g, products);
    click(&mut g, "Platform");
    texts.extend(render(&mut g).into_iter().map(|c| c.text));
    g.dialog = Some(Dialog::Survey);
    texts.extend(render(&mut g).into_iter().map(|c| c.text));
    for text in texts {
        assert!(text.is_ascii(), "non-ASCII text drawn: {text:?}");
    }
}

#[test]
fn submenus_open_beside_their_parent_instead_of_replacing_it() {
    let mut g = Gallery::default();
    let products = first(&mut g, "Products");
    press_release(&mut g, products);
    assert_eq!(g.horizontal_menu.open, vec![0]);
    let platform = find(&mut g, "Platform");
    click(&mut g, "Platform");
    assert_eq!(g.horizontal_menu.open, vec![0, 0]);
    let commands = render(&mut g);
    // The parent menu is still there, and the submenu opens to its right.
    assert!(commands.iter().any(|c| c.text == "Integrations"));
    let analytics = commands
        .iter()
        .find(|c| c.text == "Analytics")
        .unwrap()
        .rect;
    assert!(analytics.x >= platform.right());
    click(&mut g, "Analytics");
    assert_eq!(g.selected, "analytics");
    assert!(!g.horizontal_menu.is_open());

    let solutions = first(&mut g, "Solutions");
    press_release(&mut g, solutions);
    click(&mut g, "Enterprise");
    assert!(!g.vertical_menu.is_open());
    assert_eq!(g.horizontal_menu.open, vec![1, 0]);
    assert!(render(&mut g)
        .iter()
        .any(|c| c.text == "Security & Compliance"));
}

#[test]
fn vertical_submenus_cascade_to_the_right() {
    let mut g = Gallery::default();
    // The vertical menu's Products is the second one drawn.
    let rect = render(&mut g)
        .iter()
        .filter(|c| c.text == "Products")
        .nth(1)
        .unwrap()
        .rect;
    press_release(&mut g, rect);
    assert_eq!(g.vertical_menu.open, vec![0]);
    let platform = render(&mut g)
        .iter()
        .rev()
        .find(|c| c.text == "Platform")
        .unwrap()
        .rect;
    assert!(platform.x > rect.right());
    press_release(&mut g, platform);
    assert_eq!(g.vertical_menu.open, vec![0, 0]);
}

#[test]
fn theme_dropdown_changes_native_state() {
    let mut g = Gallery::default();
    click(&mut g, "White");
    click(&mut g, "Black");
    assert!(g.dark);
    assert!(!render(&mut g).iter().any(|c| c.text == "White"));
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
    g.frame(Input::default(), W, H, 3.1);
    assert!(g.timeout.is_none());
}

#[test]
fn text_input_and_checkbox_update_state() {
    let mut g = Gallery::default();
    section(&mut g, "2. Form Controls");
    click(&mut g, "Enter username...");
    type_text(&mut g, "Ada");
    assert_eq!(g.username, "Ada");
    click(&mut g, "Accept terms and conditions");
    assert!(g.accepted);
    click(&mut g, "Rust");
    click(&mut g, "Python");
    assert_eq!(LANGUAGES[g.language], "Python");
}

#[test]
fn the_calendar_icon_sits_at_the_fields_right_edge() {
    let mut g = Gallery::default();
    let label = find(&mut g, "2026-07-16 18:00");
    let commands = render(&mut g);
    let text = commands
        .iter()
        .find(|c| c.text == "2026-07-16 18:00" && c.rect == label)
        .unwrap();
    assert_eq!(text.style.text_align, TextAlign::Left);
    // The field is the button behind the label, 500 wide.
    let field = commands
        .iter()
        .find(|c| {
            c.kind == "button"
                && c.rect.width == 500.0
                && c.rect.contains(Point::new(label.x, label.y + 1.0))
        })
        .unwrap()
        .rect;
    let icon = commands
        .iter()
        .filter(|c| {
            c.kind == "triangles"
                && field.contains(Point::new(c.rect.x, c.rect.y))
                && c.rect.x > field.x + field.width * 0.5
        })
        .max_by(|a, b| a.rect.right().total_cmp(&b.rect.right()))
        .unwrap()
        .rect;
    let gap = field.right() - icon.right();
    assert!((8.0..=14.0).contains(&gap), "icon is {gap} from the edge");
}

#[test]
fn calendar_cancel_preserves_value_and_apply_commits() {
    let mut g = Gallery::default();
    click(&mut g, "2026-07-16 18:00");
    assert!(g.date_picker.open);
    assert_eq!(g.date_picker.calendar.month, 7);
    click(&mut g, "Cancel");
    assert!(!g.date_picker.open);
    assert_eq!(g.date, "2026-07-16 18:00");
    click(&mut g, "2026-07-16 18:00");
    g.date_picker.calendar.change_month(1);
    click(&mut g, "Apply");
    assert_eq!(g.date, "2026-08-16 18:00");
    assert!(!g.date_picker.open);
}

#[test]
fn color_popover_edits_and_closes() {
    let mut g = Gallery::default();
    click(&mut g, "#3b82f6");
    assert!(g.color_field.open);
    // The popover's tabs are drawn after the inline picker's.
    let presets = render(&mut g)
        .iter()
        .rev()
        .find(|c| c.text == "Presets")
        .unwrap()
        .rect;
    press_release(&mut g, presets);
    let swatch = render(&mut g)
        .iter()
        .filter(|c| c.kind == "panel" && c.style.fill == DANGER)
        .last()
        .unwrap()
        .rect;
    press_release(&mut g, swatch);
    assert_eq!(g.color, "#ef4444");
    click(&mut g, "Done");
    assert!(!g.color_field.open);
}

#[test]
fn rich_editor_formats_selection_and_saves_only_on_commit() {
    let mut g = Gallery::default();
    let original = g.rich.saved.clone();
    let select_all = |g: &mut Gallery| {
        g.frame(
            Input {
                select_all: true,
                ..Default::default()
            },
            W,
            H,
            0.0,
        );
    };
    click(&mut g, "Edit");
    select_all(&mut g);
    click(&mut g, "B");
    assert!(g.rich.draft.document.runs.iter().all(|r| r.format.bold));
    assert_eq!(g.rich.saved, original);
    click(&mut g, "Cancel");
    assert_eq!(g.rich.saved, original);
    click(&mut g, "Edit");
    select_all(&mut g);
    type_text(&mut g, "Updated 👩‍💻");
    click(&mut g, "Save");
    assert_eq!(g.rich.saved.text(), "Updated 👩‍💻");
}

#[test]
fn modal_blocks_background_and_scroll() {
    let mut g = Gallery::default();
    render(&mut g);
    g.dialog = Some(Dialog::Modal);
    let scroll = g.scroll;
    g.scroll_by(400.0);
    assert_eq!(g.scroll, scroll);
    // The page's theme dropdown is under the scrim and does not open.
    click(&mut g, "White");
    assert!(!render(&mut g).iter().any(|c| c.text == "Black"));
    click(&mut g, "Cancel");
    assert!(g.dialog.is_none());
}

#[test]
fn survey_branches_on_role_and_submits_only_visible_answers() {
    let mut g = Gallery::default();
    click(&mut g, "Launch Dynamic Form Demo");
    assert_eq!(g.dialog, Some(Dialog::Survey));
    click(&mut g, "Continue");
    assert_eq!(g.survey.step, 1);
    click(&mut g, "UI/UX Designer");
    click(&mut g, "Continue");
    assert_eq!(g.survey.step, 3);
    click(&mut g, "Submit");
    let result = g.survey_result.clone().unwrap();
    assert_eq!(result.role, "designer");
    assert!(result.tech_stack.is_empty());
    assert!(g.dialog.is_none());
}

#[test]
fn table_loading_covers_the_rows_and_blocks_sorting() {
    let mut g = Gallery::default();
    click(&mut g, "Loading");
    assert!(g.table.loading);
    let commands = render(&mut g);
    // The last "Loading" is the overlay's, not the option's checkbox.
    let loading = commands
        .iter()
        .rev()
        .find(|c| c.text == "Loading")
        .unwrap()
        .rect;
    let name = commands.iter().find(|c| c.text == "Name").unwrap().rect;
    let alice = commands
        .iter()
        .find(|c| c.text == "Alice Smith")
        .unwrap()
        .rect;
    // The last panel over the rows is the overlay: exactly the four rows.
    let overlay = commands
        .iter()
        .filter(|c| c.kind == "panel" && c.rect.contains(Point::new(alice.x, alice.y + 1.0)))
        .last()
        .unwrap()
        .rect;
    let header_bottom = name.y + name.height;
    assert!((overlay.y - header_bottom).abs() < 0.5);
    assert!((overlay.height - 4.0 * 44.0).abs() < 0.5);
    // The spinner and label are centred over it.
    let middle = overlay.y + overlay.height * 0.5;
    assert!((loading.y + loading.height * 0.5 - middle).abs() < 1.0);
    assert!(loading.x > overlay.x + overlay.width * 0.3);
    let sort = g.sort;
    click(&mut g, "Name");
    assert_eq!(g.sort, sort);
}

#[test]
fn dragging_a_pipeline_card_shows_where_it_goes() {
    let mut g = Gallery::default();
    let card = find(&mut g, "Research customer needs");
    let target = find(&mut g, "Review");
    let (x, y) = (card.x + 20.0, card.y + 10.0);
    frame_at(&mut g, x, y, false);
    frame_at(&mut g, x, y, true);
    let (tx, ty) = (target.x + 20.0, target.y + 80.0);
    let commands = frame_at(&mut g, tx, ty, true);
    assert!(g.pipeline_state.dragging());
    assert!(commands.iter().any(|c| c.text == "Move here"));
    // The card follows the pointer above the page.
    let floating = commands
        .iter()
        .filter(|c| c.text == "Research customer needs")
        .map(|c| c.layer)
        .max()
        .unwrap();
    assert!(floating > ui_kit_immediate::Layer::BASE);
    frame_at(&mut g, tx, ty, false);
    assert!(!g.pipeline_state.dragging());
    assert_eq!(g.pipeline[0].column, 3);
}

#[test]
fn collapsing_a_milestone_gives_back_its_rows() {
    let mut g = Gallery::default();
    render(&mut g);
    let height = g.content_height;
    click(&mut g, "Foundation");
    render(&mut g);
    assert!(!g.timeline[0].milestones[0].expanded);
    assert_eq!(g.content_height, height - 2.0 * 44.0);
    assert!(!render(&mut g).iter().any(|c| c.text == "Publish alpha"));
    click(&mut g, "Design system");
    render(&mut g);
    assert_eq!(g.content_height, height - 4.0 * 44.0);
}

#[test]
fn the_donut_label_is_centred_on_the_ring() {
    let mut g = Gallery::default();
    let rect = find(&mut g, "75%");
    let commands = render(&mut g);
    let index = commands
        .iter()
        .position(|c| c.text == "75%" && c.rect == rect)
        .unwrap();
    assert_eq!(commands[index].style.text_align, TextAlign::Center);
    // The ring's three slices are the meshes just before the label.
    let ring = commands[..index]
        .iter()
        .rev()
        .filter(|c| c.kind == "triangles")
        .take(3)
        .map(|c| c.rect)
        .reduce(|a, b| {
            let x = a.x.min(b.x);
            let y = a.y.min(b.y);
            Rect::new(
                x,
                y,
                a.right().max(b.right()) - x,
                a.bottom().max(b.bottom()) - y,
            )
        })
        .unwrap();
    let center = |r: Rect| (r.x + r.width * 0.5, r.y + r.height * 0.5);
    let (cx, cy) = center(ring);
    let (lx, ly) = center(rect);
    assert!((cx - lx).abs() < 1.0 && (cy - ly).abs() < 1.0);
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
        assert_eq!(parameters.len() % 2, 0);
    }
}

#[test]
fn scrollbar_drag_tracks_pointer_clamps_and_releases() {
    let mut g = Gallery::default();
    render(&mut g);
    let thumb = (H * H / g.content_height).max(24.0);
    frame_at(&mut g, 1195.0, 10.0, true);
    assert_eq!(g.scroll, 0.0);
    frame_at(&mut g, 1195.0, 200.0, true);
    let expected = 190.0 / (H - thumb) * (g.content_height - H);
    assert!((g.scroll - expected).abs() < 0.1);
    frame_at(&mut g, 1195.0, 2000.0, true);
    assert_eq!(g.scroll, g.content_height - H);
    frame_at(&mut g, 1195.0, 2000.0, false);
    frame_at(&mut g, 1195.0, 20.0, false);
    assert_eq!(g.scroll, g.content_height - H);
    g.dialog = Some(Dialog::Modal);
    frame_at(&mut g, 1195.0, 10.0, true);
    assert_eq!(g.scroll, g.content_height - H);
}

#[test]
fn flow_pan_moves_geometry_in_the_current_frame() {
    let mut g = Gallery::default();
    let before = find(&mut g, "1. Checkout");
    // Empty stage below the first node and left of the tests node.
    let (x, y) = (before.x + 150.0, before.y + 100.0);
    frame_at(&mut g, x, y, false);
    frame_at(&mut g, x, y, true);
    let moved = frame_at(&mut g, x + 60.0, y + 20.0, true);
    let after = moved.iter().find(|c| c.text == "1. Checkout").unwrap().rect;
    assert!((after.x - before.x - 60.0).abs() < 0.1);
    assert!((after.y - before.y - 20.0).abs() < 0.1);
}

#[test]
fn hierarchy_editor_adds_and_deletes_through_the_toolbar() {
    let mut g = Gallery::default();
    click(&mut g, "Editable");
    assert!(g.tree_edit);
    click(&mut g, "Web Views");
    click(&mut g, "Add child");
    let id = g.tree.selected;
    assert!(g
        .tree
        .nodes
        .iter()
        .any(|n| n.id == id && n.label == "New node"));
    click(&mut g, "Delete");
    assert!(!g.tree.nodes.iter().any(|n| n.id == id));
}
