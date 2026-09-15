pub fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

pub fn render_markdown_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut table = String::new();
    table.push_str("| ");
    table.push_str(&headers.join(" | "));
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
    fence.push_str(&format!("```{clean_lang}\n"));
    if clean_lang == "vox" && skip_doctest {
        fence.push_str("// vox:skip empirical sandbox probe\n");
    }
    fence.push_str(code.trim());
    fence.push_str("\n```\n");
    fence
}
