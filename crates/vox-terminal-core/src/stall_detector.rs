//! Interactive prompt and alternate-screen TUI stall detector for stateful PTY commands.
//!
//! Avoids false positives on slow compilers/test runners by requiring both:
//! 1. `silence_ms >= threshold_ms` (default `1_500ms`) for primary-screen prompts, AND
//! 2. a non-newline-terminated trailing prompt line matching known interactive patterns
//!    (unless `alt_screen == true`, which immediately classifies as [`PromptKind::InteractiveTui`]).

use crate::corpus::redact::scrub_ansi_and_redact;

/// Default output quiescence window (`1.5s`) before inspecting a trailing line for prompt markers.
pub const DEFAULT_STALL_QUIESCENCE_MS: u64 = 1_500;

/// Maximum UTF-8 byte length retained in [`StallSignal::prompt_tail`].
const MAX_PROMPT_TAIL_BYTES: usize = 200;

/// Classification of an interactive terminal stall awaiting input or TUI dismissal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptKind {
    YesNo,
    Credential,
    Selection,
    PressEnter,
    InteractiveTui,
}

/// Detected stall metadata emitted when a running PTY command requires human/agent action.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StallSignal {
    pub prompt_kind: PromptKind,
    pub prompt_tail: String,
    pub credential_prompt: bool,
    pub silence_ms: u64,
}

const CREDENTIAL_PATTERNS: &[&str] = &[
    "password:",
    "password for",
    "passphrase",
    "[sudo]",
    "enter pin",
    "verification code",
    "one-time code",
];

const YES_NO_PATTERNS: &[&str] = &[
    "[y/n]",
    "(y/n)",
    "[yes/no]",
    "(yes/no)",
    "continue?",
    "proceed?",
    "overwrite?",
    "are you sure",
];

const PRESS_ENTER_PATTERNS: &[&str] = &["press enter", "press any key", "hit enter"];

const SELECTION_PATTERNS: &[&str] = &["select an option", "choose ["];

fn cap_tail_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut start = text.len() - max_bytes;
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_string()
}

/// Classify whether a running PTY command has stalled on an interactive prompt or full-screen TUI.
pub fn classify_stall(
    plain_output: &str,
    alt_screen: bool,
    silence_ms: u64,
    threshold_ms: u64,
) -> Option<StallSignal> {
    if alt_screen {
        return Some(StallSignal {
            prompt_kind: PromptKind::InteractiveTui,
            prompt_tail: "[alternate screen active]".to_string(),
            credential_prompt: false,
            silence_ms,
        });
    }

    if silence_ms < threshold_ms || plain_output.is_empty() || plain_output.ends_with('\n') {
        return None;
    }

    let raw_tail = plain_output.rsplit('\n').next().unwrap_or(plain_output);
    let scrubbed = scrub_ansi_and_redact(raw_tail);
    let trimmed = scrubbed.trim_end_matches([' ', '\t']);
    if trimmed.is_empty() {
        return None;
    }

    let prompt_tail = cap_tail_utf8(trimmed, MAX_PROMPT_TAIL_BYTES);
    let lower = prompt_tail.to_ascii_lowercase();

    if CREDENTIAL_PATTERNS.iter().any(|pat| lower.contains(pat)) {
        return Some(StallSignal {
            prompt_kind: PromptKind::Credential,
            prompt_tail,
            credential_prompt: true,
            silence_ms,
        });
    }

    if YES_NO_PATTERNS.iter().any(|pat| lower.contains(pat)) {
        return Some(StallSignal {
            prompt_kind: PromptKind::YesNo,
            prompt_tail,
            credential_prompt: false,
            silence_ms,
        });
    }

    if PRESS_ENTER_PATTERNS.iter().any(|pat| lower.contains(pat)) {
        return Some(StallSignal {
            prompt_kind: PromptKind::PressEnter,
            prompt_tail,
            credential_prompt: false,
            silence_ms,
        });
    }

    if SELECTION_PATTERNS.iter().any(|pat| lower.contains(pat)) {
        return Some(StallSignal {
            prompt_kind: PromptKind::Selection,
            prompt_tail,
            credential_prompt: false,
            silence_ms,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alt_screen_triggers_immediately_without_quiescence() {
        let sig = classify_stall("", true, 0, DEFAULT_STALL_QUIESCENCE_MS)
            .expect("alt_screen should immediately classify as InteractiveTui");
        assert_eq!(sig.prompt_kind, PromptKind::InteractiveTui);
        assert!(!sig.credential_prompt);
        assert_eq!(sig.prompt_tail, "[alternate screen active]");
    }

    #[test]
    fn yes_no_prompt_requires_quiescence_threshold() {
        assert!(
            classify_stall(
                "Proceed with installation? [y/N] ",
                false,
                1_499,
                DEFAULT_STALL_QUIESCENCE_MS
            )
            .is_none()
        );

        let sig = classify_stall(
            "Proceed with installation? [y/N] ",
            false,
            1_500,
            DEFAULT_STALL_QUIESCENCE_MS,
        )
        .expect("should classify YesNo at threshold");
        assert_eq!(sig.prompt_kind, PromptKind::YesNo);
        assert!(!sig.credential_prompt);
        assert_eq!(sig.prompt_tail, "Proceed with installation? [y/N]");
    }

    #[test]
    fn credential_prompts_set_credential_flag() {
        for sample in [
            "[sudo] challenge for builder: ",
            "Enter passphrase for key_ed25519: ",
            "Password: ",
            "Enter verification code: ",
        ] {
            let sig = classify_stall(sample, false, 1_600, DEFAULT_STALL_QUIESCENCE_MS)
                .unwrap_or_else(|| panic!("expected credential stall for {sample:?}"));
            assert_eq!(sig.prompt_kind, PromptKind::Credential);
            assert!(sig.credential_prompt);
        }
    }

    #[test]
    fn press_enter_and_selection_prompts_classify_cleanly() {
        let enter_sig = classify_stall("Press Enter to continue...", false, 2_000, 1_500).unwrap();
        assert_eq!(enter_sig.prompt_kind, PromptKind::PressEnter);
        assert!(!enter_sig.credential_prompt);

        let sel_sig = classify_stall("Select an option: ", false, 2_000, 1_500).unwrap();
        assert_eq!(sel_sig.prompt_kind, PromptKind::Selection);
        assert!(!sel_sig.credential_prompt);
    }

    #[test]
    fn newline_terminated_compiler_diagnostics_never_false_positive() {
        let rustc_diag = "error[E0308]: mismatched types: are you sure? [y/N]\n";
        assert!(
            classify_stall(rustc_diag, false, 5_000, DEFAULT_STALL_QUIESCENCE_MS).is_none(),
            "newline-terminated output must never classify as an interactive prompt"
        );
    }
}
