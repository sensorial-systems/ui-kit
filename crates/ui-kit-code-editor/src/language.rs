use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammingLanguage {
    pub id: &'static str,
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub generic_extension: &'static str,
}

pub struct LanguageRegistry {
    languages: Vec<ProgrammingLanguage>,
    default_id: &'static str,
}

static REGISTRY: OnceLock<LanguageRegistry> = OnceLock::new();

impl LanguageRegistry {
    pub fn get() -> &'static Self {
        REGISTRY.get_or_init(|| {
            let languages = vec![
                ProgrammingLanguage {
                    id: "wgsl",
                    name: "WGSL",
                    extensions: &["wgsl"],
                    generic_extension: "rs", // Fallback for highlighting
                },
                ProgrammingLanguage {
                    id: "glsl",
                    name: "GLSL",
                    extensions: &["glsl", "vert", "frag", "comp"],
                    generic_extension: "c", // Fallback for highlighting
                },
            ];
            Self {
                languages,
                default_id: "wgsl",
            }
        })
    }

    pub fn default_language(&self) -> &ProgrammingLanguage {
        self.find_by_id(self.default_id).unwrap()
    }

    pub fn languages(&self) -> &[ProgrammingLanguage] {
        &self.languages
    }

    pub fn find_by_id(&self, id: &str) -> Option<&ProgrammingLanguage> {
        self.languages.iter().find(|l| l.id == id)
    }

    pub fn detect_from_code(&self, code: &str) -> Option<&ProgrammingLanguage> {
        if code.contains("#version") {
            return self.find_by_id("glsl");
        }
        // Add more detection logic here as needed
        None
    }
}
