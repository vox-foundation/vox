//! Research commands that act on the Axis host itself: saving a synthesis into the local
//! repository checkout and running a polyglot sandbox probe. Local by nature in both modes.

/// Repository root for research-doc writes: Clavis `VoxRepositoryRoot` (`VOX_REPO_ROOT`, alias
/// `VOX_REPOSITORY_ROOT`), else the current directory.
fn research_repo_root() -> std::path::PathBuf {
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxRepositoryRoot)
        .expose()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// `docs/src/architecture/` filename for a research synthesis titled `title`.
fn research_doc_filename(title: &str) -> String {
    let raw_slug = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if raw_slug.is_empty() {
        "research-topic".to_string()
    } else {
        raw_slug
    };
    if slug.ends_with("research") || slug.ends_with("findings") {
        format!("{slug}-2026.md")
    } else {
        format!("{slug}-research-2026.md")
    }
}

/// Saves a completed research synthesis into `docs/src/architecture/` with required YAML frontmatter.
#[tauri::command]
pub async fn save_research_doc(
    title: String,
    description: String,
    content: String,
    category: Option<String>,
) -> Result<String, String> {
    let doc_category = category.unwrap_or_else(|| "Architecture SSOTs".to_string());
    let frontmatter = format!(
        "---\ntitle: \"{}\"\ndescription: \"{}\"\ncategory: \"{}\"\nstatus: \"current\"\n---\n\n",
        title.replace('"', "\\\""),
        description.replace('"', "\\\""),
        doc_category.replace('"', "\\\"")
    );
    let target_dir = research_repo_root()
        .join("docs")
        .join("src")
        .join("architecture");
    let target_path = target_dir.join(research_doc_filename(&title));

    tokio::fs::create_dir_all(&target_dir)
        .await
        .map_err(|e| e.to_string())?;
    let full_text = format!("{frontmatter}{content}");
    tokio::fs::write(&target_path, full_text)
        .await
        .map_err(|e| e.to_string())?;

    Ok(target_path.to_string_lossy().to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxProbeOutcome {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

#[tauri::command]
pub async fn execute_sandbox_probe(
    code: String,
    language: String,
) -> Result<SandboxProbeOutcome, String> {
    use vox_research_shim::research::domain::polyglot_sandbox::{
        PolyglotLanguage, verify_in_polyglot_sandbox,
    };
    let lang = match language.trim().to_lowercase().as_str() {
        "python" => PolyglotLanguage::Python,
        "typescript" | "ts" => PolyglotLanguage::TypeScript,
        "sql" => PolyglotLanguage::Sql,
        "vox" => PolyglotLanguage::Vox,
        _ => PolyglotLanguage::Rust,
    };
    let res = verify_in_polyglot_sandbox(&code, lang)
        .await
        .map_err(|e| e.to_string())?;
    Ok(SandboxProbeOutcome {
        passed: res.passed,
        stdout: res.stdout,
        stderr: res.stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_filename_slugifies_and_appends_the_research_suffix_once() {
        assert_eq!(
            research_doc_filename("Mesh Routing!"),
            "mesh-routing-research-2026.md"
        );
        assert_eq!(
            research_doc_filename("Cache findings"),
            "cache-findings-2026.md"
        );
        assert_eq!(
            research_doc_filename("???"),
            "research-topic-research-2026.md"
        );
    }

    #[test]
    fn sandbox_probe_outcome_dto_serialization() {
        let outcome = SandboxProbeOutcome {
            passed: true,
            stdout: "hello".into(),
            stderr: "".into(),
        };
        let val = serde_json::to_value(&outcome).unwrap();
        assert_eq!(val["passed"], true);
        assert_eq!(val["stdout"], "hello");
        assert_eq!(val["stderr"], "");
    }
}
