#![allow(missing_docs)]

//! External integration tests for `vox-lsp` hover logic.
//!
//! These tests exercise the public `vox_lsp::builtin_hover_markdown`,
//! `builtin_hover_markdown_in_line`, `line_has_speech_transcribe`, and
//! `word_at_position` APIs through the crate boundary.

use vox_lsp::{
    builtin_hover_markdown, builtin_hover_markdown_in_line, line_has_speech_transcribe,
    word_at_position,
};

// ── word_at_position ─────────────────────────────────────────────────────────

#[test]
fn word_at_pos_extracts_identifier_at_start_of_line() {
    let text = "Speech.transcribe(p)";
    assert_eq!(word_at_position(text, 0, 0).as_deref(), Some("Speech"));
}

#[test]
fn word_at_pos_extracts_identifier_mid_word() {
    let text = "let myVar = 42";
    assert_eq!(word_at_position(text, 0, 6).as_deref(), Some("myVar"));
}

#[test]
fn word_at_pos_returns_none_on_punctuation() {
    let text = "a + b";
    assert!(word_at_position(text, 0, 2).is_none());
}

#[test]
fn word_at_pos_returns_none_past_line_end() {
    let text = "abc";
    assert!(word_at_position(text, 0, 99).is_none());
}

#[test]
fn word_at_pos_multiline_second_line() {
    let text = "fn hello():\n    greet()\n";
    assert_eq!(word_at_position(text, 1, 4).as_deref(), Some("greet"));
}

#[test]
fn word_at_pos_underscore_is_ident_char() {
    let text = "_private_fn()";
    assert_eq!(word_at_position(text, 0, 3).as_deref(), Some("_private_fn"));
}

// ── line_has_speech_transcribe ───────────────────────────────────────────────

#[test]
fn speech_transcribe_detects_plain_call() {
    assert!(line_has_speech_transcribe("Speech.transcribe(path)"));
}

#[test]
fn speech_transcribe_detects_with_whitespace() {
    assert!(line_has_speech_transcribe("Speech . transcribe (x)"));
}

#[test]
fn speech_transcribe_rejects_prefixed_receiver() {
    assert!(!line_has_speech_transcribe("MySpeech.transcribe(x)"));
    assert!(!line_has_speech_transcribe("FooSpeech.transcribe(x)"));
}

#[test]
fn speech_transcribe_rejects_standalone_transcribe() {
    assert!(!line_has_speech_transcribe("let transcribe = 1"));
    assert!(!line_has_speech_transcribe("transcribe(x)"));
}

#[test]
fn speech_transcribe_detects_in_indented_line() {
    assert!(line_has_speech_transcribe("    Speech.transcribe(file)"));
}

// ── builtin_hover_markdown ───────────────────────────────────────────────────

#[test]
fn hover_speech_returns_markdown() {
    let doc = builtin_hover_markdown("Speech").expect("Speech hover");
    assert!(doc.contains("Speech"), "missing 'Speech' in: {doc}");
    assert!(doc.contains("transcribe"), "missing 'transcribe' in: {doc}");
}

#[test]
fn hover_transcribe_returns_markdown() {
    let doc = builtin_hover_markdown("transcribe").expect("transcribe hover");
    assert!(doc.contains("transcribe"), "missing 'transcribe' in: {doc}");
}

#[test]
fn hover_http_returns_markdown() {
    let doc = builtin_hover_markdown("HTTP").expect("HTTP hover");
    assert!(doc.contains("HTTP"), "missing 'HTTP' in: {doc}");
}

#[test]
fn hover_print_returns_markdown() {
    let doc = builtin_hover_markdown("print").expect("print hover");
    assert!(doc.contains("print"), "missing 'print' in: {doc}");
}

#[test]
fn hover_unknown_symbol_returns_none() {
    assert!(builtin_hover_markdown("nonexistent_fn").is_none());
}

// ── builtin_hover_markdown_in_line ───────────────────────────────────────────

#[test]
fn hover_in_line_transcribe_with_speech_receiver() {
    let line = "    Speech.transcribe(p)";
    assert!(builtin_hover_markdown_in_line(line, "transcribe").is_some());
}

#[test]
fn hover_in_line_transcribe_without_speech_receiver_returns_none() {
    assert!(builtin_hover_markdown_in_line("other.transcribe(p)", "transcribe").is_none());
    assert!(builtin_hover_markdown_in_line("let transcribe = 1", "transcribe").is_none());
}

#[test]
fn hover_in_line_non_transcribe_word_ignores_line_context() {
    // For non-transcribe words, line context is not checked.
    let doc = builtin_hover_markdown_in_line("foo.HTTP.get()", "HTTP");
    assert!(doc.is_some(), "HTTP hover should not require Speech context");
}
