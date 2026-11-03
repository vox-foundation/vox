//! Per-file mechanical classification (spec §5.1). A change is mechanical when any
//! reason applies; the commit subject is recorded as a hint but never decides.
use super::model::Reason;
use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::ToTokens;

/// Extensions where whitespace never changes meaning. Indentation-sensitive or unknown
/// types (py, yaml, md, vox, Makefile, …) never get `Whitespace`: hiding a real change
/// costs more than showing a formatting one.
const WS_INSIGNIFICANT: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "json", "toml", "css", "html", "sql",
];

fn ws_insignificant(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| WS_INSIGNIFICANT.contains(&e))
}

pub fn classify(
    path: &str,
    status: &str,
    generated: bool,
    whitespace_only: bool,
    before: Option<&str>,
    after: Option<&str>,
) -> Vec<Reason> {
    let mut reasons = Vec::new();
    if generated {
        reasons.push(Reason::Generated);
    }
    // Only a modification can be "no meaningful change"; a rename or add is always real.
    if status != "M" {
        return reasons;
    }
    if whitespace_only && ws_insignificant(path) {
        reasons.push(Reason::Whitespace);
    }
    if path.ends_with(".rs") {
        if let (Some(b), Some(a)) = (
            before.and_then(rust_normalized),
            after.and_then(rust_normalized),
        ) {
            if b == a {
                reasons.push(Reason::SymbolNeutral);
            }
        }
    }
    reasons
}

/// Token text of a Rust file with `use` items and `#[doc]` / `#[derive]` attributes
/// removed, so formatting, import churn, derive churn, and doc edits normalize away.
/// `None` when the file does not parse.
pub fn rust_normalized(src: &str) -> Option<String> {
    let mut file = syn::parse_file(src).ok()?;
    strip_uses(&mut file.items);
    Some(strip_attrs(file.to_token_stream()).to_string())
}

fn strip_uses(items: &mut Vec<syn::Item>) {
    items.retain(|i| !matches!(i, syn::Item::Use(_)));
    for item in items.iter_mut() {
        if let syn::Item::Mod(m) = item {
            if let Some((_, inner)) = &mut m.content {
                strip_uses(inner);
            }
        }
    }
}

fn strip_attrs(ts: TokenStream) -> TokenStream {
    let mut out: Vec<TokenTree> = Vec::new();
    let mut toks = ts.into_iter().peekable();
    while let Some(t) = toks.next() {
        if matches!(&t, TokenTree::Punct(p) if p.as_char() == '#') {
            let mut look = toks.clone();
            let bang = matches!(look.peek(), Some(TokenTree::Punct(b)) if b.as_char() == '!');
            if bang {
                look.next();
            }
            if let Some(TokenTree::Group(g)) = look.peek() {
                if g.delimiter() == Delimiter::Bracket && is_doc_or_derive(g.stream()) {
                    if bang {
                        toks.next();
                    }
                    toks.next();
                    continue;
                }
            }
        }
        out.push(match t {
            TokenTree::Group(g) => {
                TokenTree::Group(Group::new(g.delimiter(), strip_attrs(g.stream())))
            }
            other => other,
        });
    }
    out.into_iter().collect()
}

fn is_doc_or_derive(attr: TokenStream) -> bool {
    matches!(attr.into_iter().next(), Some(TokenTree::Ident(i)) if i == "doc" || i == "derive")
}

/// What a commit subject claims. Recorded only; v1 never uses it to set `mechanical`.
pub fn subject_hint(subject: &str) -> Option<&'static str> {
    let lower = subject.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |f: &dyn Fn(&str) -> bool| words.iter().any(|w| f(w));
    if has(&|w| w == "hakari") {
        Some("hakari")
    } else if has(&|w| w == "fmt" || w == "rustfmt" || w.starts_with("format")) {
        Some("fmt")
    } else if has(&|w| w == "clippy" || w == "lint" || w == "lints") {
        Some("lint")
    } else if has(&|w| w == "regen" || w == "regenerate" || w == "resync") {
        Some("regen")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neutral(b: &str, a: &str) -> bool {
        classify("x.rs", "M", false, false, Some(b), Some(a)).contains(&Reason::SymbolNeutral)
    }

    #[test]
    fn formatting_imports_derives_and_docs_are_symbol_neutral() {
        assert!(neutral("fn a() -> u8 { 1 }", "fn a()->u8{\n    1\n}\n"));
        assert!(neutral(
            "use a::b;\nuse c::d;\nfn f() {}",
            "use c::d;\nuse a::b;\nfn f() {}"
        ));
        assert!(neutral(
            "#[derive(Debug)]\nstruct S;",
            "#[derive(Debug, Clone)]\nstruct S;"
        ));
        assert!(neutral("/// old\nfn f() {}", "/// new doc\nfn f() {}"));
        assert!(neutral(
            "mod m { use x::y; fn g() {} }",
            "mod m { fn g() {} }"
        ));
    }

    #[test]
    fn body_changes_and_unparseable_files_are_not_neutral() {
        assert!(!neutral("fn a() -> u8 { 1 }", "fn a() -> u8 { 2 }"));
        assert!(!neutral("fn a() {", "fn a() {}"));
        assert!(
            !neutral("#[cfg(test)]\nfn a() {}", "fn a() {}"),
            "non-doc attrs matter"
        );
    }

    #[test]
    fn generated_and_whitespace_flags_and_non_rust() {
        assert_eq!(
            classify("Cargo.lock", "M", true, false, None, None),
            vec![Reason::Generated]
        );
        assert_eq!(
            classify("a.ts", "M", false, true, Some("x"), Some("x ")),
            vec![Reason::Whitespace]
        );
        // AMENDED: #20 — valid Rust text in a .ts file, so deleting the `.rs` check fails this test.
        assert!(
            classify(
                "a.ts",
                "M",
                false,
                false,
                Some("fn f() {}"),
                Some("fn  f(){}")
            )
            .is_empty(),
            "symbol check is Rust-only"
        );
    }

    #[test]
    fn renames_and_adds_are_never_mechanical_unless_generated() {
        let src = "fn f() {}\n";
        assert!(classify("b.rs", "R", false, false, Some(src), Some(src)).is_empty());
        assert!(classify("b.rs", "A", false, true, None, Some(src)).is_empty());
        assert_eq!(
            classify("b.lock", "R", true, false, None, None),
            vec![Reason::Generated]
        );
    }

    #[test]
    fn indentation_sensitive_files_never_get_whitespace() {
        assert!(
            classify(
                "a.py",
                "M",
                false,
                true,
                Some("if x:\n    y()\n"),
                Some("if x:\ny()\n")
            )
            .is_empty()
        );
        assert!(
            classify(
                "c.yaml",
                "M",
                false,
                true,
                Some("a:\n  b: 1\n"),
                Some("a:\nb: 1\n")
            )
            .is_empty()
        );
        assert!(
            classify("Makefile", "M", false, true, None, None).is_empty(),
            "no extension = not allowlisted"
        );
    }

    #[test]
    fn subject_hint_matches_words_not_substrings() {
        assert_eq!(subject_hint("chore(hakari): resync"), Some("hakari"));
        assert_eq!(subject_hint("style: rustfmt pass"), Some("fmt"));
        assert_eq!(subject_hint("fix clippy lints"), Some("lint"));
        assert_eq!(
            subject_hint("chore(ssot): regenerate registry"),
            Some("regen")
        );
        assert_eq!(subject_hint("feat: show more information"), None);
    }
}
