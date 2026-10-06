use serde::{Deserialize, Serialize};

/// Represents source code with a string buffer.
/// This type is designed to be used with `DataType::Any` and `Value::new_any`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Code {
    source: String,
}

impl Code {
    /// Creates a new Code instance with the given source.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
        }
    }

    /// Returns a reference to the source code.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Sets the source code.
    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = source.into();
    }

    /// Returns the length of the source code in bytes.
    pub fn len(&self) -> usize {
        self.source.len()
    }

    /// Returns whether the source code is empty.
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }
}

impl Default for Code {
    fn default() -> Self {
        Self {
            source: String::new(),
        }
    }
}

impl From<String> for Code {
    fn from(source: String) -> Self {
        Self::new(source)
    }
}

impl From<&str> for Code {
    fn from(source: &str) -> Self {
        Self::new(source)
    }
}

impl From<Code> for String {
    fn from(code: Code) -> Self {
        code.source
    }
}

impl AsRef<str> for Code {
    fn as_ref(&self) -> &str {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_creation() {
        let code = Code::new("fn main() {}");
        assert_eq!(code.source(), "fn main() {}");
    }

    #[test]
    fn test_code_mutation() {
        let mut code = Code::new("old code");
        code.set_source("new code");
        assert_eq!(code.source(), "new code");
    }

    #[test]
    fn test_code_default() {
        let code = Code::default();
        assert!(code.is_empty());
        assert_eq!(code.len(), 0);
    }

    #[test]
    fn test_code_from_string() {
        let code: Code = "test".into();
        assert_eq!(code.source(), "test");
    }
}
