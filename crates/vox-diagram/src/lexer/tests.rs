use super::*;
use crate::codes;

fn run(src: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    lex(src, &LineIndex::new(src))
}

fn kinds(src: &str) -> Vec<TokenKind> {
    run(src).0.into_iter().map(|t| t.kind).collect()
}

fn id(s: &str) -> TokenKind {
    TokenKind::Ident(s.into())
}

fn st(s: &str) -> TokenKind {
    TokenKind::Str {
        value: s.into(),
        terminated: true,
    }
}

#[test]
fn lexes_edge_and_string_with_byte_spans() {
    let (toks, diags) = run("a -> \"é\"\r\n");
    let k: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        k,
        vec![
            id("a"),
            TokenKind::Arrow,
            st("é"),
            TokenKind::Newline,
            TokenKind::Eof
        ]
    );
    assert!(diags.is_empty());
    assert_eq!(toks[2].span, Span { start: 5, end: 9 });
    assert_eq!(toks[3].span, Span { start: 9, end: 11 });
}

#[test]
fn kebab_idents_stop_before_arrows() {
    assert_eq!(
        kinds("api-gw -> db"),
        vec![id("api-gw"), TokenKind::Arrow, id("db"), TokenKind::Eof]
    );
    assert_eq!(
        kinds("a->b"),
        vec![id("a"), TokenKind::Arrow, id("b"), TokenKind::Eof]
    );
    // `a-->b`: the dash run is left for the parser.
    assert_eq!(kinds("a-->b")[0], id("a"));
}

#[test]
fn string_escapes_decode() {
    let (toks, diags) = run(r#""say \"hi\" \\ x\nnext""#);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(toks[0].kind, st("say \"hi\" \\ x\nnext"));
}

#[test]
fn invalid_escape_is_one_error_and_keeps_literal_text() {
    let (toks, diags) = run(r#""bad \t""#);
    assert_eq!(toks[0].kind, st("bad \\t"));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, codes::INVALID_ESCAPE);
    assert_eq!((diags[0].span.start, diags[0].span.end), (5, 7));
}

#[test]
fn forbidden_char_is_reported_per_char_with_exact_span() {
    let (_, diags) = run("\"a\u{202e}b\u{7}\"");
    assert_eq!(diags.len(), 2);
    assert!(diags.iter().all(|d| d.code == codes::FORBIDDEN_CHAR));
    assert_eq!((diags[0].span.start, diags[0].span.end), (2, 5));
    assert_eq!((diags[1].span.start, diags[1].span.end), (6, 7));
    let (_, ok) = run("\"a\tb\"");
    assert!(ok.is_empty(), "a raw tab is allowed: {ok:?}");
}

#[test]
fn unterminated_string_does_not_cascade() {
    let (toks, diags) = run("\"open\nnext");
    assert_eq!(
        toks.into_iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::Str {
                value: "open".into(),
                terminated: false
            },
            TokenKind::Newline,
            id("next"),
            TokenKind::Eof
        ]
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, codes::UNTERMINATED_STRING);
}

#[test]
fn bad_words_are_one_invalid_identifier_each() {
    for word in ["9lives", "école", "caf\u{e9}"] {
        let (toks, diags) = run(word);
        assert_eq!(toks[0].kind, TokenKind::BadIdent(word.into()), "{word}");
        assert_eq!(diags.len(), 1, "{word}");
        assert_eq!(diags[0].code, codes::INVALID_IDENTIFIER);
        assert_eq!(diags[0].span.end as usize, word.len());
    }
}

#[test]
fn comments_crlf_and_bom() {
    let (toks, diags) = run("\u{feff}// note  \r\nx");
    assert!(diags.is_empty());
    let k: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        k,
        vec![
            TokenKind::Comment(" note".into()),
            TokenKind::Newline,
            id("x"),
            TokenKind::Eof
        ]
    );
    assert_eq!(toks[0].span, Span { start: 3, end: 10 });
}

#[test]
fn stray_chars_become_unknown() {
    assert_eq!(kinds("?"), vec![TokenKind::Unknown('?'), TokenKind::Eof]);
}

fn only_fix(diag: &Diagnostic) -> (u32, u32, &str) {
    assert_eq!(diag.fixes.len(), 1, "{diag:?}");
    let f = &diag.fixes[0];
    (f.span.start, f.span.end, f.replacement.as_str())
}

#[test]
fn lexical_fixes_are_exact() {
    // Insert the missing quote after the trimmed text, not after trailing blanks.
    let (_, d) = run("\"open  \nnext");
    assert_eq!(only_fix(&d[0]), (5, 5, "\""));
    // Double the backslash, keep the escaped char.
    let (_, d) = run(r#""bad \t""#);
    assert_eq!(only_fix(&d[0]), (5, 6, "\\\\"));
    // Delete the forbidden char (3 bytes), nothing else.
    let (_, d) = run("\"a\u{202e}b\"");
    assert_eq!(only_fix(&d[0]), (2, 5, ""));
    // A trailing backslash is both an invalid escape and an unterminated string; the two
    // fixes touch (`\` replaced) and (`"` inserted after it) without overlapping.
    let (_, d) = run("\"x\\");
    let fixes: Vec<_> = d.iter().map(only_fix).collect();
    assert!(
        fixes.contains(&(2, 3, "\\\\")) && fixes.contains(&(3, 3, "\"")),
        "{fixes:?}"
    );
}
