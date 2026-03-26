//! Ensures `contracts/speech-to-code/vox_grammar_artifact.json` token slices still lex as Vox tokens.

use std::path::PathBuf;

use vox_compiler::lexer::cursor::lex;
use vox_compiler::lexer::token::Token;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn assert_lexes_as_single_token(slice: &str, label: &str) {
    let toks: Vec<Token> = lex(slice).into_iter().map(|s| s.token).collect();
    assert!(
        toks.len() >= 2,
        "{label}: expected tokens + EOF for {slice:?}, got {toks:?}"
    );
    assert!(
        !matches!(toks[0], Token::Ident(_)),
        "{label}: {slice:?} should not lex as bare Ident, got {:?}",
        toks[0]
    );
    assert!(
        matches!(toks[toks.len() - 1], Token::Eof),
        "{label}: expected EOF sentinel"
    );
}

#[test]
fn vox_grammar_artifact_keywords_lex() {
    let root = workspace_root();
    let raw = std::fs::read_to_string(root.join("contracts/speech-to-code/vox_grammar_artifact.json"))
        .expect("read vox_grammar_artifact.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse artifact json");
    for key in ["keywords", "decorators", "punctuators"] {
        let arr = v
            .get(key)
            .and_then(|x| x.as_array())
            .unwrap_or_else(|| panic!("missing array {key}"));
        for item in arr {
            let s = item.as_str().expect("string token");
            assert_lexes_as_single_token(s, key);
        }
    }
}
