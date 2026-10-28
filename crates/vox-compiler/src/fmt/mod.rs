//! Pretty-print / minimally format Vox source by round-tripping parse → string.
//!
//! - [`format()`] returns the original buffer when parsing fails (editor-safe soft mode).
//! - [`try_format`] is **fail-closed**: returns parse/round-trip errors for tooling (`vox fmt`).

mod expr;
mod printer;
mod stmt;

use crate::lexer::lex;
use crate::parser::{ParseError, parse};

fn layout_only(source: &str, out: &str) -> bool {
    use crate::lexer::{Token, lex_preserving};
    let strip = |s: &str| -> Vec<Token> {
        lex_preserving(s)
            .into_iter()
            .map(|s| s.token)
            .filter(|t| !matches!(t, Token::Newline | Token::Eof))
            .collect()
    };
    strip(source) == strip(out)
}

/// Format `source` when it parses cleanly; otherwise return `source` unchanged.
pub fn format(source: &str) -> String {
    let tokens = lex(source);
    let module = match parse(tokens) {
        Ok(m) => m,
        Err(_) => return source.to_string(), // Incomplete/invalid source - return as-is
    };

    let mut printer = printer::Printer::new();
    printer.print_module(&module);
    let out = printer.finish().trim_end().to_string() + "\n";
    if !layout_only(source, &out) {
        return source.to_string();
    }
    out
}

/// Parse, print, and **re-parse** output. Fails if the source is invalid or the printer loses validity.
pub fn try_format(source: &str) -> Result<String, Vec<ParseError>> {
    let tokens = lex(source);
    let module = parse(tokens)?;
    let mut printer = printer::Printer::new();
    printer.print_module(&module);
    let out = printer.finish().trim_end().to_string() + "\n";
    if !layout_only(source, &out) {
        return Err(vec![ParseError::new(
            crate::ast::span::Span::new(0, 0),
            "formatter would change more than layout (comments or constructs it cannot print yet); file left unchanged",
            vec![],
            None,
        )]);
    }
    parse(lex(&out)).map(|_| ())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assert formatting is idempotent: format(format(x)) == format(x)
    fn assert_idempotent(source: &str) {
        let once = format(source);
        let twice = format(&once);
        assert_eq!(once, twice, "Formatting is not idempotent for:\n{source}");
    }

    #[test]
    fn test_format_environment() {
        let source = r#"
@environment production:
    base: "node:22-alpine"
    workdir: "/app"
    packages: ["curl"]
    env:
        NODE_ENV: "production"
    expose: [3000]
    volumes: ["/data"]
    copy:
        "nginx.conf" to "/etc/nginx/nginx.conf"
    run: ["echo 'Building...'"]
    cmd: ["npm", "start"]
"#;
        let formatted = format(source);
        let expected = r#"@environment production:
    base: "node:22-alpine"
    workdir: "/app"
    packages: ["curl"]
    env:
        NODE_ENV: "production"
    expose: [3000]
    volumes: ["/data"]
    copy:
        "nginx.conf" to "/etc/nginx/nginx.conf"
    run: ["echo 'Building...'"]
    cmd: ["npm", "start"]
"#;
        assert_eq!(formatted.trim(), expected.trim());
    }

    #[test]
    fn idempotent_simple_fn() {
        assert_idempotent(
            "fn add(x: int, y: int) to int {
    return x + y
}\n",
        );
    }

    #[test]
    fn try_format_matches_soft_format_when_valid() {
        let src = "fn add(x: int, y: int) to int {\n    return x + y\n}\n";
        let soft = format(src);
        let strict = try_format(src).expect("try_format");
        assert_eq!(soft, strict);
    }

    #[test]
    fn idempotent_table_decl() {
        assert_idempotent(
            "table Note {
    title: str
    content: str
    created_at: str
}\n",
        );
    }

    #[test]
    fn idempotent_server_fn() {
        assert_idempotent(
            "server greet(name: str) to str {
    return \"hello\"
}\n",
        );
    }

    #[test]
    fn idempotent_query_fn() {
        assert_idempotent(
            "query list_items() to list[Item] {
    return []
}\n",
        );
    }

    #[test]
    fn idempotent_mutation_fn() {
        assert_idempotent(
            "mutation add_item(name: str) to Result[str] {
    return Ok(name)
}\n",
        );
    }

    /// The printer does not keep comments; formatting must refuse rather than delete
    /// them (`vox fmt` once stripped golden frontmatter and `// EXPECT:` markers).
    #[test]
    fn formatter_refuses_to_drop_comments() {
        let src = "// keep me\nfn f() to int {\n    return 1\n}\n";
        let err = try_format(src).expect_err("dropping a comment must be refused");
        assert!(err[0].message.contains("layout"), "{:?}", err[0].message);
        assert_eq!(
            format(src),
            src,
            "editor format must leave the buffer unchanged"
        );
    }

    /// A construct the printer cannot print must not vanish: `vox fmt` once deleted a
    /// whole `query` declaration from `examples/golden/db_native_ir.vox`.
    #[test]
    fn formatter_refuses_to_drop_declarations() {
        let src = "query q() to int {\n    return 1\n}\n";
        assert!(
            try_format(src).is_err(),
            "dropping a declaration must be refused"
        );
        assert_eq!(format(src), src);
    }

    /// Every infix operator must survive a round trip: the printer once emitted `??`
    /// for any operator it lacked (e.g. `>` in `assert(x > 1.0)`), so `vox fmt` failed.
    #[test]
    fn every_binop_round_trips() {
        let src = "fn ops(a: int, b: int, p: bool, q: bool) to bool {
    let lt = a < b
    let gt = a > b
    let le = a <= b
    let ge = a >= b
    let m = a % b
    let s = a + b - a * b / b
    let both = p and q
    let either = p or q
    return lt is gt
}\n";
        let out = try_format(src).expect("every operator must print and re-parse");
        for op in [" < ", " > ", " <= ", " >= ", " % ", " and ", " or ", " is "] {
            assert!(out.contains(op), "lost `{}` in:\n{out}", op.trim());
        }
        assert!(
            !out.contains("??"),
            "printer emitted a placeholder operator:\n{out}"
        );
        assert_idempotent(src);
    }

    /// `->` match arms are deprecated syntax; the printer must emit `=>`.
    #[test]
    fn match_arms_print_fat_arrow() {
        let src = "fn f(n: int) to int {
    return match n {
        0 => 1
        _ => 2
    }
}\n";
        let out = try_format(src).expect("match must round-trip");
        assert!(out.contains("0 => 1"), "{out}");
        assert!(!out.contains(" -> "), "printed deprecated `->` arm:\n{out}");
    }

    #[test]
    fn idempotent_import() {
        assert_idempotent("import react.use_state\n");
    }

    #[test]
    fn idempotent_const() {
        assert_idempotent("const MAX: int = 100\n");
    }

    #[test]
    fn idempotent_for_loop() {
        assert_idempotent(
            "fn process(items: list[str]) to int {
    for item in items {
        return 0
    }
    return 1
}\n",
        );
    }

    #[test]
    fn idempotent_workflow() {
        assert_idempotent(
            "workflow my_flow(input: str) to Result[str] {
    return Ok(input)
}\n",
        );
    }

    #[test]
    fn idempotent_actor() {
        assert_idempotent(
            "actor Counter {
    on increment(n: int) to int {
        return n
    }
}\n",
        );
    }

    #[test]
    fn idempotent_routes() {
        assert_idempotent(
            "routes {
    \"/\" to Home
    \"/about\" to About
}\n",
        );
    }

    #[test]
    fn table_uses_brace_syntax() {
        let out = format(
            "table User {
    name: str
    age: int
}\n",
        );
        assert!(
            out.contains("table User {"),
            "expected brace block, got: {out}"
        );
        assert!(out.contains('{'), "must use brace syntax, got: {out}");
    }

    #[test]
    fn format_invalid_source_returns_original() {
        let broken = "fn () { !!! }";
        let out = format(broken);
        assert_eq!(out, broken, "Invalid source should be returned unchanged");
    }
}
