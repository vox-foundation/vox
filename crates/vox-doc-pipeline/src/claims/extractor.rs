//! Deterministic AST mention extractor for documentation markdown files.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use super::types::{ExtractedMention, MentionKind, MentionRole};

static RE_FILE_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?:crates|docs|contracts|\.planning|scripts|tests|examples|assets|docs-astro)/[a-zA-Z0-9_.-]+(?:/[a-zA-Z0-9_.-]+)*|Cargo\.toml|README\.md|AGENTS\.md)$").unwrap()
});

static RE_CRATE_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:vox-[a-z0-9]+(?:-[a-z0-9]+)*)$").unwrap());

static RE_ENV_VAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:VOX_[A-Z0-9_]+|RUST_LOG|RUST_BACKTRACE|TURSO_[A-Z0-9_]+)$").unwrap()
});

static RE_CONFIG_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_]*(?:\.[a-z0-9_-]+)+$").unwrap());

static RE_RUST_SYMBOL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*(?:::_?[a-zA-Z0-9_]+)+$").unwrap());

/// Line indexer for mapping byte offsets to 1-based line numbers.
struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (idx, b) in text.as_bytes().iter().enumerate() {
            if *b == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self { line_starts }
    }

    fn line_number(&self, byte_offset: usize) -> usize {
        match self.line_starts.binary_search(&byte_offset) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }
}

/// Extract all concrete code/repo mentions from a Markdown document.
pub fn extract_mentions(source_file: &Path, content: &str) -> Vec<ExtractedMention> {
    let mut mentions = Vec::new();
    let line_index = LineIndex::new(content);
    let source_path_str = source_file.to_string_lossy().to_string();

    let mut current_fence: Option<String> = None;
    let mut fence_is_skip = false;
    let mut fence_is_text = false;

    let parser = Parser::new_ext(content, Options::all()).into_offset_iter();

    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let info_str = info.as_ref().trim().to_lowercase();
                fence_is_text = info_str == "text" || info_str == "plaintext";
                current_fence = Some(info_str);
                fence_is_skip = false;
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                current_fence = Some("indented".to_string());
                fence_is_skip = false;
                fence_is_text = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                current_fence = None;
                fence_is_skip = false;
                fence_is_text = false;
            }
            Event::Code(code) => {
                let raw = code.as_ref().trim();
                let byte_offset = range.start;
                let line_number = line_index.line_number(byte_offset);

                let role = if is_descriptive_context(content, byte_offset) {
                    MentionRole::Mentions
                } else {
                    MentionRole::Prescribes
                };

                if let Some(mention) =
                    classify_inline_code(raw, role, &source_path_str, line_number, byte_offset)
                {
                    mentions.push(mention);
                }
            }
            Event::Text(text) => {
                let text_str = text.as_ref();
                if let Some(ref fence) = current_fence {
                    if text_str.contains("// vox:skip") || text_str.contains("# vox:skip") {
                        fence_is_skip = true;
                    }

                    let role = if fence_is_skip || fence_is_text {
                        MentionRole::Mentions
                    } else {
                        MentionRole::Prescribes
                    };

                    extract_from_code_block(
                        text_str,
                        fence,
                        role,
                        &source_path_str,
                        range.start,
                        &line_index,
                        &mut mentions,
                    );
                }
            }
            _ => {}
        }
    }

    mentions
}

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Check if the text surrounding the inline code on the same line indicates an illustrative or historical mention.
fn is_descriptive_context(content: &str, byte_offset: usize) -> bool {
    let end = floor_char_boundary(content, byte_offset);
    let start = floor_char_boundary(content, byte_offset.saturating_sub(90));
    let line_end = content[end..]
        .find('\n')
        .map(|i| end + i)
        .unwrap_or(content.len());
    let slice = content[start..line_end].to_lowercase();
    slice.contains("retired")
        || slice.contains("deprecated")
        || slice.contains("replaced by")
        || slice.contains("superseded")
        || slice.contains("historical")
        || slice.contains("formerly")
        || slice.contains("previously")
        || slice.contains("deleted")
        || slice.contains("removed")
        || slice.contains("archived")
        || slice.contains("planned")
        || slice.contains("proposed")
        || slice.contains("roadmap")
        || slice.contains("e.g.")
        || slice.contains("for example")
}

fn is_placeholder_target(raw: &str) -> bool {
    raw.contains("...")
        || raw.contains('…')
        || raw.contains("YYYY")
        || raw.contains("XXXX")
        || raw.ends_with("_X")
        || raw.contains('<')
        || raw.contains('>')
        || raw.contains("/foo")
        || raw.contains("/bar")
        || raw.contains("nonexistent")
        || raw.contains("fake")
}

fn classify_inline_code(
    raw: &str,
    role: MentionRole,
    source_file: &str,
    line_number: usize,
    byte_offset: usize,
) -> Option<ExtractedMention> {
    if raw.is_empty() {
        return None;
    }
    let effective_role = if is_placeholder_target(raw) {
        MentionRole::Mentions
    } else {
        role
    };

    // 1. File Path
    if RE_FILE_PATH.is_match(raw) {
        return Some(ExtractedMention {
            kind: MentionKind::FilePath,
            role: effective_role,
            raw_text: raw.to_string(),
            normalized_target: raw.to_string(),
            source_file: source_file.to_string(),
            line_number,
            byte_offset,
            fence_info: None,
        });
    }

    // 2. CLI Command
    if raw.starts_with("vox ") || raw.starts_with("cargo run -p vox") {
        let normalized = raw.trim().to_string();
        return Some(ExtractedMention {
            kind: MentionKind::CliCommand,
            role: effective_role,
            raw_text: raw.to_string(),
            normalized_target: normalized,
            source_file: source_file.to_string(),
            line_number,
            byte_offset,
            fence_info: None,
        });
    }

    // 3. Environment Variable
    if RE_ENV_VAR.is_match(raw) {
        return Some(ExtractedMention {
            kind: MentionKind::EnvVar,
            role: effective_role,
            raw_text: raw.to_string(),
            normalized_target: raw.to_string(),
            source_file: source_file.to_string(),
            line_number,
            byte_offset,
            fence_info: None,
        });
    }

    // 4. Crate Name
    if RE_CRATE_NAME.is_match(raw) {
        return Some(ExtractedMention {
            kind: MentionKind::CrateName,
            role: effective_role,
            raw_text: raw.to_string(),
            normalized_target: raw.to_string(),
            source_file: source_file.to_string(),
            line_number,
            byte_offset,
            fence_info: None,
        });
    }

    // 5. Rust Symbol
    if RE_RUST_SYMBOL.is_match(raw) && !raw.contains('/') {
        return Some(ExtractedMention {
            kind: MentionKind::RustSymbol,
            role: effective_role,
            raw_text: raw.to_string(),
            normalized_target: raw.to_string(),
            source_file: source_file.to_string(),
            line_number,
            byte_offset,
            fence_info: None,
        });
    }

    // 6. Config Key
    if RE_CONFIG_KEY.is_match(raw)
        && !raw.contains('/')
        && !raw.ends_with(".rs")
        && !raw.ends_with(".md")
    {
        return Some(ExtractedMention {
            kind: MentionKind::ConfigKey,
            role: effective_role,
            raw_text: raw.to_string(),
            normalized_target: raw.to_string(),
            source_file: source_file.to_string(),
            line_number,
            byte_offset,
            fence_info: None,
        });
    }

    None
}

fn extract_from_code_block(
    block_text: &str,
    fence_info: &str,
    role: MentionRole,
    source_file: &str,
    block_start_offset: usize,
    line_index: &LineIndex,
    mentions: &mut Vec<ExtractedMention>,
) {
    let mut current_offset = block_start_offset;
    for line in block_text.lines() {
        let line_trimmed = line.trim();
        let line_num = line_index.line_number(current_offset);

        // CLI command in code fence
        if line_trimmed.starts_with("vox ")
            || line_trimmed.starts_with("$ vox ")
            || line_trimmed.starts_with("cargo run -p vox")
        {
            let cmd_text = line_trimmed.trim_start_matches('$').trim();
            let cmd_role = if is_placeholder_target(cmd_text) {
                MentionRole::Mentions
            } else {
                role
            };
            mentions.push(ExtractedMention {
                kind: MentionKind::CliCommand,
                role: cmd_role,
                raw_text: cmd_text.to_string(),
                normalized_target: cmd_text.to_string(),
                source_file: source_file.to_string(),
                line_number: line_num,
                byte_offset: current_offset,
                fence_info: Some(fence_info.to_string()),
            });
        }

        // Env vars in code fence (e.g. export VOX_FOO=bar or VOX_FOO: "...")
        for word in line_trimmed.split(&[' ', '=', '"', '\'', ':', ';', '(', ')'][..]) {
            if RE_ENV_VAR.is_match(word) {
                let env_role = if is_placeholder_target(word) {
                    MentionRole::Mentions
                } else {
                    role
                };
                mentions.push(ExtractedMention {
                    kind: MentionKind::EnvVar,
                    role: env_role,
                    raw_text: word.to_string(),
                    normalized_target: word.to_string(),
                    source_file: source_file.to_string(),
                    line_number: line_num,
                    byte_offset: current_offset,
                    fence_info: Some(fence_info.to_string()),
                });
            }
        }

        // Advance offset
        current_offset += line.len() + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn extracts_inline_code_mentions() {
        let doc = r#"
# Sample Doc
Run `vox audit docs` to inspect claims.
The configuration uses `VOX_DB_URL` for database access.
See implementation in `crates/vox-cli/src/main.rs` and crate `vox-orchestrator`.
Symbol `vox_actor_runtime::llm` provides inference.
Retired alias `TURSO_URL` should be avoided.
"#;
        let mentions = extract_mentions(Path::new("docs/src/test.md"), doc);
        assert!(!mentions.is_empty());

        let has_cmd = mentions
            .iter()
            .any(|m| m.kind == MentionKind::CliCommand && m.raw_text == "vox audit docs");
        let has_env = mentions
            .iter()
            .any(|m| m.kind == MentionKind::EnvVar && m.raw_text == "VOX_DB_URL");
        let has_path = mentions
            .iter()
            .any(|m| m.kind == MentionKind::FilePath && m.raw_text == "crates/vox-cli/src/main.rs");
        let has_crate = mentions
            .iter()
            .any(|m| m.kind == MentionKind::CrateName && m.raw_text == "vox-orchestrator");
        let has_sym = mentions
            .iter()
            .any(|m| m.kind == MentionKind::RustSymbol && m.raw_text == "vox_actor_runtime::llm");
        let retired_role = mentions
            .iter()
            .find(|m| m.raw_text == "TURSO_URL")
            .unwrap()
            .role;

        assert!(has_cmd);
        assert!(has_env);
        assert!(has_path);
        assert!(has_crate);
        assert!(has_sym);
        assert_eq!(retired_role, MentionRole::Mentions);
    }

    #[test]
    fn honors_vox_skip_in_fences() {
        let doc = r#"
```vox
// vox:skip: test excerpt
fn test() {
    let url = "VOX_DB_URL";
}
```

```text
vox audit docs
```
"#;
        let mentions = extract_mentions(Path::new("docs/src/test.md"), doc);
        let skip_mention = mentions
            .iter()
            .find(|m| m.raw_text == "VOX_DB_URL")
            .unwrap();
        assert_eq!(skip_mention.role, MentionRole::Mentions);

        let text_mention = mentions
            .iter()
            .find(|m| m.raw_text == "vox audit docs")
            .unwrap();
        assert_eq!(text_mention.role, MentionRole::Mentions);
    }
}
