/// Namespace management for the content-addressed store.
pub struct Namespace {
    segments: Vec<String>,
}

impl Namespace {
    /// Empty namespace (displays as `.`).
    pub fn root() -> Self {
        Self { segments: vec![] }
    }

    /// Parse a dot-separated path into segment components.
    pub fn new(path: &str) -> Self {
        Self {
            segments: path.split('.').map(|s| s.to_string()).collect(),
        }
    }

    /// Append one segment, producing a deeper namespace.
    pub fn child(&self, name: &str) -> Self {
        let mut segments = self.segments.clone();
        segments.push(name.to_string());
        Self { segments }
    }

    /// Parent namespace, or `None` if already at the root.
    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            None
        } else {
            let mut segments = self.segments.clone();
            segments.pop();
            Some(Self { segments })
        }
    }
}

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.segments.is_empty() {
            write!(f, ".")
        } else {
            write!(f, "{}", self.segments.join("."))
        }
    }
}
