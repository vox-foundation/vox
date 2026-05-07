/// A simple brace-depth tracker for JSON generation.
///
/// This is used to track the depth of nested objects and arrays to ensure
/// that the generated JSON is structurally balanced.
#[derive(Debug, Clone, Default)]
pub struct JsonBraceDepthTracker {
    in_string: bool,
    brace_depth: usize,
}

impl JsonBraceDepthTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a new chunk of string into the tracker.
    /// Returns true if the string is structurally valid (so far), false if it causes a brace imbalance.
    pub fn is_valid_transition(&self, chunk: &str) -> bool {
        if chunk.is_empty() {
            return true;
        }

        let mut temp_state = self.clone();
        for c in chunk.chars() {
            if !temp_state.transition_char(c) {
                return false;
            }
        }
        true
    }

    pub fn advance(&mut self, chunk: &str) {
        for c in chunk.chars() {
            self.transition_char(c);
        }
    }

    fn transition_char(&mut self, c: char) -> bool {
        if self.in_string {
            if c == '"' {
                self.in_string = false;
            }
            return true;
        }

        match c {
            '{' | '[' => {
                self.brace_depth += 1;
                true
            }
            '}' | ']' => {
                if self.brace_depth == 0 {
                    return false;
                }
                self.brace_depth -= 1;
                true
            }
            '"' => {
                self.in_string = true;
                true
            }
            _ => true, // ignore other characters outside strings for brace tracking
        }
    }
}
