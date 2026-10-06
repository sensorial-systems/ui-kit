use ropey::Rope;

/// A text buffer backed by a rope data structure for efficient editing.
/// Tracks a monotonically increasing version number for incremental updates.
#[derive(Clone, Debug)]
pub struct TextBuffer {
    rope: Rope,
    version: u64,
}

impl TextBuffer {
    /// Creates a new empty text buffer.
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            version: 0,
        }
    }

    /// Creates a text buffer from the given text.
    pub fn from_text(text: impl AsRef<str>) -> Self {
        Self {
            rope: Rope::from_str(text.as_ref()),
            version: 0,
        }
    }

    /// Returns the current version of the buffer.
    /// The version increments on every edit operation.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns the total length of the text in bytes.
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Returns the total length of the text in characters.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Returns the number of lines in the buffer.
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Returns whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.rope.len_bytes() == 0
    }

    /// Returns the entire text as a String.
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Returns a slice of text from the given byte range.
    pub fn slice_bytes(&self, start: usize, end: usize) -> String {
        self.rope.byte_slice(start..end).to_string()
    }

    /// Returns the text of a specific line (0-indexed).
    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx >= self.len_lines() {
            return None;
        }
        Some(self.rope.line(line_idx).to_string())
    }

    /// Returns the byte offset of the start of the given line.
    pub fn line_to_byte(&self, line_idx: usize) -> Option<usize> {
        if line_idx >= self.len_lines() {
            return None;
        }
        Some(self.rope.line_to_byte(line_idx))
    }

    /// Returns the line index containing the given byte offset.
    pub fn byte_to_line(&self, byte_idx: usize) -> Option<usize> {
        if byte_idx > self.len_bytes() {
            return None;
        }
        Some(self.rope.byte_to_line(byte_idx))
    }

    /// Inserts text at the given byte offset.
    /// Returns the new version number.
    pub fn insert(&mut self, byte_idx: usize, text: &str) -> u64 {
        self.rope.insert(byte_idx, text);
        self.version += 1;
        self.version
    }

    /// Removes text in the given byte range.
    /// Returns the new version number.
    pub fn remove(&mut self, start: usize, end: usize) -> u64 {
        self.rope.remove(start..end);
        self.version += 1;
        self.version
    }

    /// Replaces text in the given byte range with new text.
    /// Returns the new version number.
    pub fn replace(&mut self, start: usize, end: usize, text: &str) -> u64 {
        self.rope.remove(start..end);
        self.rope.insert(start, text);
        self.version += 1;
        self.version
    }

    /// Returns a reference to the underlying rope.
    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for TextBuffer {
    fn from(text: String) -> Self {
        Self::from_text(text)
    }
}

impl From<&str> for TextBuffer {
    fn from(text: &str) -> Self {
        Self::from_text(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_buffer() {
        let buffer = TextBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len_bytes(), 0);
        assert_eq!(buffer.version(), 0);
    }

    #[test]
    fn test_from_text() {
        let buffer = TextBuffer::from_text("hello\nworld");
        assert_eq!(buffer.len_lines(), 2);
        assert_eq!(buffer.to_string(), "hello\nworld");
    }

    #[test]
    fn test_insert() {
        let mut buffer = TextBuffer::from_text("hello");
        let v1 = buffer.insert(5, " world");
        assert_eq!(buffer.to_string(), "hello world");
        assert_eq!(v1, 1);
        assert_eq!(buffer.version(), 1);
    }

    #[test]
    fn test_remove() {
        let mut buffer = TextBuffer::from_text("hello world");
        let v1 = buffer.remove(5, 11);
        assert_eq!(buffer.to_string(), "hello");
        assert_eq!(v1, 1);
    }

    #[test]
    fn test_replace() {
        let mut buffer = TextBuffer::from_text("hello world");
        buffer.replace(6, 11, "Rust");
        assert_eq!(buffer.to_string(), "hello Rust");
    }

    #[test]
    fn test_line_access() {
        let buffer = TextBuffer::from_text("line1\nline2\nline3");
        assert_eq!(buffer.line(0), Some("line1\n".to_string()));
        assert_eq!(buffer.line(1), Some("line2\n".to_string()));
        assert_eq!(buffer.line(2), Some("line3".to_string()));
        assert_eq!(buffer.line(3), None);
    }

    #[test]
    fn test_byte_line_conversion() {
        let buffer = TextBuffer::from_text("abc\ndef\nghi");
        assert_eq!(buffer.byte_to_line(0), Some(0));
        assert_eq!(buffer.byte_to_line(4), Some(1));
        assert_eq!(buffer.byte_to_line(8), Some(2));

        assert_eq!(buffer.line_to_byte(0), Some(0));
        assert_eq!(buffer.line_to_byte(1), Some(4));
        assert_eq!(buffer.line_to_byte(2), Some(8));
    }

    #[test]
    fn test_version_tracking() {
        let mut buffer = TextBuffer::new();
        assert_eq!(buffer.version(), 0);

        buffer.insert(0, "a");
        assert_eq!(buffer.version(), 1);

        buffer.insert(1, "b");
        assert_eq!(buffer.version(), 2);

        buffer.remove(0, 1);
        assert_eq!(buffer.version(), 3);
    }
}
