pub fn escape_pipe(s: &str) -> String {
    s.replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
        .replace('|', "\\|")
}

pub fn render_markdown_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut table = String::new();
    table.push_str("| ");
    let escaped_headers: Vec<String> = headers.iter().map(|h| escape_pipe(h)).collect();
    table.push_str(&escaped_headers.join(" | "));
    table.push_str(" |\n| ");
    table.push_str(
        &headers
            .iter()
            .map(|_| ":---")
            .collect::<Vec<_>>()
            .join(" | "),
    );
    table.push_str(" |\n");

    for row in rows {
        table.push_str("| ");
        let escaped: Vec<String> = row.iter().map(|c| escape_pipe(c)).collect();
        table.push_str(&escaped.join(" | "));
        table.push_str(" |\n");
    }
    table
}

pub fn render_code_fence(lang: &str, code: &str, skip_doctest: bool) -> String {
    let mut fence = String::new();
    let clean_lang = lang.to_lowercase();

    let mut backtick_count = 3;
    while code.contains(&"`".repeat(backtick_count)) {
        backtick_count += 1;
    }
    let delimiter = "`".repeat(backtick_count);

    fence.push_str(&format!("{delimiter}{clean_lang}\n"));
    if clean_lang == "vox" && skip_doctest {
        fence.push_str("// vox:skip empirical sandbox probe\n");
    }
    fence.push_str(code.trim());
    fence.push_str(&format!("\n{delimiter}\n"));
    fence
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_pipe_in_file() {
        assert_eq!(escape_pipe("a|b\nc"), "a\\|b c");
    }

    #[test]
    fn test_render_markdown_table_in_file() {
        let headers = ["H1", "H2"];
        let rows = vec![vec!["A".to_string(), "B".to_string()]];
        let res = render_markdown_table(&headers, &rows);
        assert!(res.contains("| H1 | H2 |"));
    }

    #[test]
    fn test_render_code_fence_in_file() {
        let fence = render_code_fence("vox", "fn main() {}", true);
        assert!(fence.contains("// vox:skip"));
    }
}
