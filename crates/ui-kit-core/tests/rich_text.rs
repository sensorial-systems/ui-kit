use ui_kit_core::{FormatAction, RichText, RichTextEditor, TextAlign};
#[test]
fn edits_preserve_format_and_undo() {
    let mut e = RichTextEditor::new(RichText::plain("hello world"));
    e.anchor = 6;
    e.cursor = 11;
    e.format(FormatAction::Bold);
    assert!(e.document.runs.last().unwrap().format.bold);
    e.insert("Rust");
    assert_eq!(e.document.text(), "hello Rust");
    assert!(e.document.runs.last().unwrap().format.bold);
    e.undo();
    assert_eq!(e.document.text(), "hello world");
    e.undo();
    assert_eq!(e.document.runs.len(), 1);
    e.redo();
    assert!(e.document.runs.last().unwrap().format.bold);
}
#[test]
fn deletion_and_navigation_use_graphemes() {
    let mut e = RichTextEditor::new(RichText::plain("A👩‍💻e\u{301}"));
    e.backspace();
    assert_eq!(e.document.text(), "A👩‍💻");
    e.move_by(-1, false);
    assert_eq!(e.cursor, 1);
    e.delete();
    assert_eq!(e.document.text(), "A");
    e.undo();
    assert_eq!(e.document.text(), "A👩‍💻");
}
#[test]
fn selection_formatting_and_html_are_safe() {
    let mut e = RichTextEditor::new(RichText::plain("<script>&\""));
    e.select_all();
    e.format(FormatAction::Italic);
    e.format(FormatAction::Underline);
    e.align(TextAlign::Right);
    let html = e.document.to_html();
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;&amp;&quot;"));
    assert!(
        html.contains("italic") && html.contains("underline") && html.contains("text-align:right")
    );
}
#[test]
fn list_and_alignment_participate_in_undo() {
    let mut e = RichTextEditor::new(RichText::plain("one\ntwo"));
    e.select_all();
    e.list(true);
    assert_eq!(e.document.text(), "1. one\n2. two");
    e.undo();
    assert_eq!(e.document.text(), "one\ntwo");
}
