use glyphon::cosmic_text as ct;
use ui_kit_core::{RichText, TextAlign};
pub(crate) fn fill(buffer: &mut ct::Buffer, doc: &RichText, size: f32) {
    buffer.set_rich_text(
        doc.runs.iter().map(|run| {
            let f = run.format;
            let scale = match f.heading {
                1 => 2.0,
                2 => 1.5,
                _ => 1.0,
            };
            let mut attrs = ct::Attrs::new()
                .weight(ct::Weight(if f.bold || f.heading > 0 { 700 } else { 400 }))
                .style(if f.italic {
                    ct::Style::Italic
                } else {
                    ct::Style::Normal
                })
                .metrics(ct::Metrics::new(size * scale, size * scale * 1.25));
            if f.underline {
                attrs = attrs.underline(ct::UnderlineStyle::Single);
            }
            if f.strike {
                attrs = attrs.strikethrough();
            }
            (run.text.as_str(), attrs)
        }),
        &ct::Attrs::new(),
        ct::Shaping::Advanced,
        Some(match doc.align {
            TextAlign::Left => ct::Align::Left,
            TextAlign::Center => ct::Align::Center,
            TextAlign::Right => ct::Align::Right,
        }),
    );
}
pub(crate) fn cursor(text: &str, index: usize) -> ct::Cursor {
    let mut line = 0;
    let mut byte = 0;
    for c in text.chars().take(index) {
        if c == '\n' {
            line += 1;
            byte = 0;
        } else {
            byte += c.len_utf8();
        }
    }
    ct::Cursor::new(line, byte)
}
fn index(text: &str, cursor: ct::Cursor) -> usize {
    text.split('\n')
        .take(cursor.line)
        .map(|l| l.chars().count() + 1)
        .sum::<usize>()
        + text
            .split('\n')
            .nth(cursor.line)
            .unwrap_or("")
            .get(..cursor.index)
            .unwrap_or("")
            .chars()
            .count()
}
/// CPU-only text shaping for hit testing. Uses the same fonts and layout as GPU painting.
pub struct RichTextLayout {
    fonts: ct::FontSystem,
}
impl Default for RichTextLayout {
    fn default() -> Self {
        Self {
            fonts: ct::FontSystem::new(),
        }
    }
}
impl RichTextLayout {
    pub fn buffer(&mut self, doc: &RichText, width: f32, height: f32, size: f32) -> ct::Buffer {
        let mut b = ct::Buffer::new(&mut self.fonts, ct::Metrics::new(size, size * 1.25));
        b.set_size(Some(width.max(1.0)), Some(height.max(1.0)));
        fill(&mut b, doc, size);
        b.shape_until_scroll(&mut self.fonts, false);
        b
    }
    pub fn hit(
        &mut self,
        doc: &RichText,
        width: f32,
        height: f32,
        size: f32,
        x: f32,
        y: f32,
    ) -> usize {
        let buffer = self.buffer(doc, width, height, size);
        buffer
            .hit(x, y)
            .map_or(doc.len(), |c| index(&doc.text(), c))
    }
    pub fn vertical(
        &mut self,
        doc: &RichText,
        width: f32,
        height: f32,
        size: f32,
        position: usize,
        down: bool,
    ) -> usize {
        let buffer = self.buffer(doc, width, height, size);
        let text = doc.text();
        let c = cursor(&text, position);
        let (x, y) = buffer.cursor_position(&c).unwrap_or((0.0, 0.0));
        buffer
            .hit(x, y + size * 1.25 * if down { 1.5 } else { -0.5 })
            .map_or(position, |c| index(&text, c))
    }
}
