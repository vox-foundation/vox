#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsmState {
    Start,
    InsideObject,
    Key,
    Colon,
    StringValue,
    NumberValue,
    Comma,
    End,
    Invalid,
}

/// A rudimentary deterministic state machine for JSON generation.
#[derive(Debug, Clone)]
pub struct JsonGrammarAutomaton {
    #[allow(dead_code)]
    state: FsmState,
    in_string: bool,
    brace_depth: usize,
}

impl JsonGrammarAutomaton {
    pub fn new() -> Self {
        Self {
            state: FsmState::Start,
            in_string: false,
            brace_depth: 0,
        }
    }

    /// Feeds a new chunk of string (e.g. decoded from the latest token) into the automaton.
    /// Returns true if the string can transition to a valid state, false if it causes a syntax error.
    /// This is a simplified fallback that mimics a basic JSON parser's state transitions.
    pub fn is_valid_transition(&self, chunk: &str) -> bool {
        // If empty chunk (sometimes tokenizers emit empty strings), it's harmless
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

        if c.is_whitespace() {
            return true;
        }

        match c {
            '{' => {
                self.brace_depth += 1;
                true
            }
            '}' => {
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
            '[' | ']' => true,                                           // arrays
            ':' | ',' => true,                                           // separators
            '0'..='9' | '-' | '.' | 'e' | 'E' | '+' => true,             // numbers
            't' | 'r' | 'u' | 'f' | 'a' | 'l' | 's' | 'n' | 'o' => true, // literals
            _ => false, // invalid character outside string
        }
    }
}
