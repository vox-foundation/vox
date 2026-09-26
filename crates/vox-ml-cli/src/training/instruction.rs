//! Instruction pair templates and name extraction.

/// Instruction templates keyed by construct type.
/// Each entry is a list of template strings where `{name}` is replaced
/// with the primary identifier extracted from the code.
pub fn instruction_templates(construct: &str) -> &[&str] {
    match construct {
        "function" => &[
            "Write a Vox function called {name}",
            "Implement the {name} function using Vox syntax",
        ],
        "component" => &[
            "Write a Vox UI component called {name}",
            "Create a {name} component in Vox",
        ],
        "actor" => &[
            "Write a Vox actor called {name} with state management",
            "Create a {name} actor in Vox using the actor model",
        ],
        "workflow" => &[
            "Write a Vox durable workflow called {name}",
            "Create a {name} workflow in Vox with retry policies",
        ],
        "activity" => &[
            "Write a Vox activity called {name}",
            "Create a retryable {name} activity in Vox",
        ],
        "table" => &[
            "Define a Vox database table called {name}",
            "Create a {name} table in Vox",
        ],
        "query" => &[
            "Write a Vox database query called {name}",
            "Create a read-only query {name} in Vox",
        ],
        "mutation" => &[
            "Write a Vox database mutation called {name}",
            "Create a {name} mutation in Vox for data modification",
        ],
        "action" => &["Write a Vox server action called {name}"],
        "type" => &[
            "Define a Vox tagged union type called {name}",
            "Create a {name} ADT in Vox with typed variants",
        ],
        "test" => &[
            "Write a Vox test for {name}",
            "Create unit tests in Vox using @test and assert",
        ],
        "mcp_tool" => &[
            "Write a Vox MCP tool called {name}",
            "Create an MCP-compatible tool in Vox for AI assistants",
        ],
        "mcp_resource" => &["Write a Vox MCP resource for {name}"],
        "http_route" => &[
            "Write an HTTP route in Vox",
            "Create an HTTP endpoint in Vox",
        ],
        "routes" => &["Define client-side routes in Vox"],
        "server_fn" => &["Write a Vox server function called {name}"],
        "skill" => &["Write a Vox skill called {name}"],
        "agent_def" => &["Define a Vox AI agent called {name}"],
        "trait" => &["Define a Vox trait called {name}"],
        _ => &["Write Vox code demonstrating {name}"],
    }
}

/// Split golden-file metadata out of training code.
///
/// Returns `(code, training_prompt)`: `code` has the leading `// ---` frontmatter
/// block, `// ANCHOR:` / `// ANCHOR_END:` markers and `// @training_prompt:` lines
/// removed (they are file metadata, not Vox a model should learn to emit);
/// `training_prompt` is the author-written task from `// @training_prompt:`.
pub fn split_training_metadata(code: &str) -> (String, Option<String>) {
    let mut out = Vec::new();
    let mut prompt = None;
    let mut lines = code.lines().peekable();
    while lines.peek().is_some_and(|l| l.trim().is_empty()) {
        lines.next();
    }
    if lines.peek().is_some_and(|l| l.trim() == "// ---") {
        lines.next();
        for l in lines.by_ref() {
            if l.trim() == "// ---" {
                break;
            }
        }
    }
    for line in lines {
        let t = line.trim_start();
        if let Some(p) = t.strip_prefix("// @training_prompt:") {
            prompt = Some(p.trim().to_string()).filter(|p| !p.is_empty());
        } else if !(t.starts_with("// ANCHOR:") || t.starts_with("// ANCHOR_END:")) {
            out.push(line);
        }
    }
    (out.join("\n").trim().to_string(), prompt)
}

/// Extract the primary declared name from a Vox source string, or `None`.
///
/// Skips comments and decorators; understands every bare-keyword declaration
/// (`fn`, `pub fn`, `component`, `table`, `query`, `mutation`, `server`,
/// `tool "desc" name`, …) so prompts stop falling back to a placeholder name.
pub fn extract_name_from_source(code: &str) -> Option<String> {
    const KEYWORDS: &[&str] = &[
        "fn",
        "component",
        "actor",
        "type",
        "workflow",
        "activity",
        "table",
        "query",
        "mutation",
        "server",
        "tool",
        "resource",
        "state_machine",
        "module",
        "trait",
        "agent",
        "skill",
        "message",
        "form",
        "index",
    ];
    let ident = |s: &str| -> Option<String> {
        let n: String = s
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        (!n.is_empty() && !n.starts_with(|c: char| c.is_ascii_digit())).then_some(n)
    };
    for line in code.lines() {
        let mut t = line.trim();
        if t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        // Strip leading decorators (`@auth(scheme: bearer) table ...`) and `pub`.
        while let Some(rest) = t.strip_prefix('@') {
            let end = rest.find(' ').unwrap_or(rest.len());
            let paren = rest.find('(').filter(|&p| p < end);
            t = match paren {
                Some(_) => rest.find(") ").map_or("", |i| &rest[i + 2..]),
                None => &rest[end..],
            }
            .trim_start();
        }
        t = t.strip_prefix("pub ").unwrap_or(t);
        for kw in KEYWORDS {
            let Some(rest) = t.strip_prefix(kw).filter(|r| r.starts_with(' ')) else {
                continue;
            };
            // `tool "name: description" fn_name(...)`, `resource "uri" "desc" name()`
            let mut r = rest.trim_start();
            while let Some(q) = r.strip_prefix('"') {
                r = q.find('"').map_or("", |i| q[i + 1..].trim_start());
            }
            if let Some(n) = ident(r) {
                return Some(n);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_names_from_bare_keyword_declarations() {
        let cases = [
            ("// fn not_this()\nfn add(a: int) to int {", "add"),
            ("pub fn run() {", "run"),
            ("component Counter(initial: int) {", "Counter"),
            ("@auth(scheme: bearer) table Task {", "Task"),
            ("query list_tasks() to List[Task] {", "list_tasks"),
            (
                "tool \"search: find docs\" search_docs(q: str) to str {",
                "search_docs",
            ),
            ("@test fn adds() {", "adds"),
            ("type MergeError =\n    | WrongKind", "MergeError"),
        ];
        for (src, want) in cases {
            assert_eq!(
                extract_name_from_source(src).as_deref(),
                Some(want),
                "{src}"
            );
        }
        assert_eq!(extract_name_from_source("import std.mobile\n"), None);
    }

    #[test]
    fn strips_golden_metadata_and_keeps_training_prompt() {
        let src = "// ---\n// title: \"X\"\n// training_eligible: true\n// ---\n// @training_prompt: Build a counter.\n\n// ANCHOR: display\n// A counter.\ncomponent C() {\n}\n// ANCHOR_END: display\n";
        let (code, prompt) = split_training_metadata(src);
        assert_eq!(code, "// A counter.\ncomponent C() {\n}");
        assert_eq!(prompt.as_deref(), Some("Build a counter."));
        let (plain, none) = split_training_metadata("fn a() {\n}\n");
        assert_eq!((plain.as_str(), none), ("fn a() {\n}", None));
    }
}
