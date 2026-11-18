//! Input box using reedline — submits via `classify()` → `Session`.

use ratatui::{Frame, layout::Rect, style::Style, widgets::Paragraph};
use vox_terminal_core::input::{InputIntent, classify};

/// Single-line input state backed by a reedline buffer.
///
/// `reedline`'s full event loop is designed for real terminals; here we use
/// only its line-buffer abstraction and drive events from crossterm ourselves.
pub struct InputBox {
    pub buf: String,
    pub ghost_suffix: String,
}

impl InputBox {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            ghost_suffix: String::new(),
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.buf.push(c);
    }

    pub fn backspace(&mut self) {
        self.buf.pop();
    }

    /// Update inline ghost-text suggestion from recent session inputs.
    pub fn update_ghost(&mut self, history: &[String]) {
        if self.buf.trim().is_empty() {
            self.ghost_suffix.clear();
            return;
        }
        let hit = history
            .iter()
            .rev()
            .find(|cmd| cmd.starts_with(&self.buf) && cmd.len() > self.buf.len());
        self.ghost_suffix = hit
            .map(|cmd| cmd[self.buf.len()..].to_string())
            .unwrap_or_default();
    }

    /// Accept the active ghost-text suffix into `buf`, returning `true` when applied.
    pub fn accept_ghost(&mut self) -> bool {
        if self.ghost_suffix.is_empty() {
            return false;
        }
        self.buf.push_str(&self.ghost_suffix);
        self.ghost_suffix.clear();
        true
    }

    /// Active input routing badge for the current buffer prefix.
    #[must_use]
    pub fn active_mode(&self) -> &'static str {
        let t = self.buf.trim_start();
        if t.starts_with("!nu ") || t == "!nu" || t.starts_with("/nu ") {
            "nu"
        } else if t.starts_with('?') || t.starts_with("/ai ") || t.starts_with("/ask ") {
            "agent"
        } else if t.starts_with('/') {
            "slash"
        } else if t.starts_with('!') {
            "shell"
        } else {
            "vox"
        }
    }

    /// Submit the current buffer and return the parsed intent; clears the buffer.
    pub fn submit(&mut self) -> InputIntent {
        let intent = classify(&self.buf);
        self.buf.clear();
        self.ghost_suffix.clear();
        intent
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, mode: &str) {
        use ratatui::{
            style::Color,
            text::{Line, Span},
        };
        let mut spans = vec![
            Span::styled(format!("[{mode}] ❯ "), Style::default()),
            Span::raw(self.buf.clone()),
            Span::raw("█"),
        ];
        if !self.ghost_suffix.is_empty() {
            spans.push(Span::styled(
                self.ghost_suffix.clone(),
                Style::default().fg(Color::DarkGray),
            ));
        }
        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line), area);
    }
}

impl Default for InputBox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_box_ghost_completion_and_mode_badges_work() {
        let mut box_state = InputBox::new();
        assert_eq!(box_state.active_mode(), "vox");
        for ch in "car".chars() {
            box_state.push_char(ch);
        }
        let history = vec![
            "cargo check -p vox-term".to_string(),
            "git status".to_string(),
        ];
        box_state.update_ghost(&history);
        assert_eq!(box_state.ghost_suffix, "go check -p vox-term");
        assert!(box_state.accept_ghost());
        assert_eq!(box_state.buf, "cargo check -p vox-term");
        assert!(box_state.ghost_suffix.is_empty());

        box_state.buf = "!nu ls".to_string();
        assert_eq!(box_state.active_mode(), "nu");
        box_state.buf = "?explain".to_string();
        assert_eq!(box_state.active_mode(), "agent");
        box_state.buf = "/budget".to_string();
        assert_eq!(box_state.active_mode(), "slash");
        box_state.buf = "!pwd".to_string();
        assert_eq!(box_state.active_mode(), "shell");
        box_state.backspace();
        assert_eq!(box_state.buf, "!pw");
        let intent = box_state.submit();
        assert!(matches!(intent, InputIntent::Shell(_)));
        assert!(box_state.buf.is_empty());
    }
}
