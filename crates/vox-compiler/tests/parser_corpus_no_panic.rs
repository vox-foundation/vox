//! Malformed inputs that must never panic the lexer + parser (A06 fuzz-style corpus).

use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;

const CORPUS: &[&str] = &[
    "",
    "fn x(",
    "fn x() to int { ret 1",
    "{{{{",
    "import \n\n",
    "@component X() { view: <div />",
    "type X =\n",
    "http post \"",
    r#" "\ "#,
    "pub pub pub",
    "fn a() to int { ret 1 }\n\x00garbage",
];

#[test]
fn parser_corpus_does_not_panic() {
    for src in CORPUS {
        let tokens = lex(src);
        let _ = parse(tokens);
    }
}
