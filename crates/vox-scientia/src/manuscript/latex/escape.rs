//! Canonical LaTeX escape for arbitrary text.
//!
//! The TeX special characters that need escaping in text mode are:
//!   `\`, `{`, `}`, `$`, `&`, `#`, `_`, `%`, `^`, `~`.
//!
//! This module implements the canonical escape recommended by the LaTeX
//! Companion (2nd ed.) for general text content. Math mode is NOT covered —
//! callers must wrap math expressions in `$...$` themselves before
//! escaping (or use the markdown-aware `render_latex` which respects
//! pulldown-cmark's code-block and inline-code spans).

/// Escape `s` for safe inclusion in LaTeX text mode.
pub fn escape_latex(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\textbackslash{}"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '$' => out.push_str(r"\$"),
            '&' => out.push_str(r"\&"),
            '#' => out.push_str(r"\#"),
            '_' => out.push_str(r"\_"),
            '%' => out.push_str(r"\%"),
            '^' => out.push_str(r"\textasciicircum{}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            _ => out.push(c),
        }
    }
    out
}

/// Normalize common Unicode punctuation and math symbols for `-no-shell-escape`
/// `pdflatex` compilation without requiring extra font/math Unicode packages.
pub fn normalize_latex_unicode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '—' => out.push_str("---"),
            '–' => out.push_str("--"),
            '±' => out.push_str(r"$\pm$"),
            '≤' => out.push_str(r"$\le$"),
            '≥' => out.push_str(r"$\ge$"),
            '×' => out.push_str(r"$\times$"),
            '→' => out.push_str(r"$\rightarrow$"),
            '−' => out.push_str("$-$"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape ASCII LaTeX specials and normalize Unicode punctuation/math symbols
/// for safe `-no-shell-escape` TeX compilation.
pub fn escape_latex_pdftex(s: &str) -> String {
    normalize_latex_unicode(&escape_latex(s))
}

/// Sanitize a verbatim code-block body so an embedded `\end{verbatim}` cannot
/// prematurely terminate the environment and Unicode symbols normalize cleanly.
pub fn sanitize_verbatim(s: &str) -> String {
    let stripped = s.replace("\\end{verbatim}", "[end-verbatim]");
    let mut out = String::with_capacity(stripped.len());
    for c in stripped.chars() {
        match c {
            '—' => out.push_str("--"),
            '–' => out.push('-'),
            '±' => out.push_str("+/-"),
            '≤' => out.push_str("<="),
            '≥' => out.push_str(">="),
            '×' => out.push('x'),
            '→' => out.push_str("->"),
            '−' => out.push('-'),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_through_ordinary_text_unchanged() {
        assert_eq!(escape_latex("Hello world."), "Hello world.");
    }

    #[test]
    fn escapes_all_ten_special_characters() {
        let input = r"\ { } $ & # _ % ^ ~";
        let out = escape_latex(input);
        // Each special should produce its escape form.
        assert!(out.contains(r"\textbackslash{}"));
        assert!(out.contains(r"\{"));
        assert!(out.contains(r"\}"));
        assert!(out.contains(r"\$"));
        assert!(out.contains(r"\&"));
        assert!(out.contains(r"\#"));
        assert!(out.contains(r"\_"));
        assert!(out.contains(r"\%"));
        assert!(out.contains(r"\textasciicircum{}"));
        assert!(out.contains(r"\textasciitilde{}"));
    }

    #[test]
    fn unicode_passes_through() {
        let s = "Café — naïve résumé";
        assert_eq!(escape_latex(s), s);
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert_eq!(escape_latex(""), "");
    }

    #[test]
    fn double_underscore_in_variable_names_escapes_twice() {
        assert_eq!(escape_latex("foo_bar_baz"), r"foo\_bar\_baz");
    }

    #[test]
    fn percent_sign_in_numbers_is_escaped() {
        assert_eq!(escape_latex("23%"), r"23\%");
    }

    #[test]
    fn pdftex_normalization_and_verbatim_sanitization() {
        let normalized = escape_latex_pdftex("10 ± 2 ≤ 12 ≥ 8 × 2 → ok — test – dash −1");
        assert!(normalized.contains(r"$\pm$"));
        assert!(normalized.contains(r"$\le$"));
        assert!(normalized.contains(r"$\ge$"));
        assert!(normalized.contains(r"$\times$"));
        assert!(normalized.contains(r"$\rightarrow$"));
        assert!(normalized.contains("---"));
        assert!(normalized.contains("--"));
        assert!(normalized.contains("$-$1"));

        let verb = sanitize_verbatim("let x = 1; \\end{verbatim} \\input{x} → ≤");
        assert!(!verb.contains("\\end{verbatim}"));
        assert!(verb.contains("[end-verbatim]"));
        assert!(verb.contains("-> <="));
    }
}
