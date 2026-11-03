use clap::Args;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Args, Debug, Clone)]
pub struct ResearchParityArgs {
    #[arg(long, default_value = "docs/src/architecture")]
    pub docs_dir: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct ParityFinding {
    schema_version: u32,
    doc_path: String,
    referenced_probe: String,
    probe_exists: bool,
}

pub fn extract_empirical_test_references(doc_content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in doc_content.split_whitespace() {
        let clean = word.trim_matches(|c| {
            matches!(
                c,
                '`' | '(' | ')' | '[' | ']' | '"' | '\'' | '*' | ',' | ':' | ';'
            )
        });
        let clean = clean.strip_suffix('.').unwrap_or(clean);
        let clean =
            clean.trim_matches(|c| matches!(c, '`' | '(' | ')' | '[' | ']' | '"' | '\'' | '*'));
        if clean.ends_with("_probe_test.rs") || clean.ends_with("_test.rs") {
            if !out.contains(&clean.to_string()) {
                out.push(clean.to_string());
            }
        }
    }
    out
}

pub fn run_research_parity_audit_sync(args: &ResearchParityArgs) -> anyhow::Result<usize> {
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let docs_dir = if args.docs_dir.is_relative() && !args.docs_dir.exists() {
        repo_root.join(&args.docs_dir)
    } else {
        args.docs_dir.clone()
    };

    if !docs_dir.exists() {
        anyhow::bail!("Docs directory not found: {:?}", docs_dir);
    }

    let mut total_probes = 0usize;
    for entry in std::fs::read_dir(&docs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let content = std::fs::read_to_string(&path)?;
            let refs = extract_empirical_test_references(&content);
            for r in refs {
                total_probes += 1;
                let exists = repo_root.join(&r).exists() || Path::new(&r).exists();
                if args.json {
                    let finding = ParityFinding {
                        schema_version: 1,
                        doc_path: path.display().to_string(),
                        referenced_probe: r,
                        probe_exists: exists,
                    };
                    println!("{}", serde_json::to_string(&finding)?);
                }
            }
        }
    }
    if !args.json {
        println!(
            "Audited architecture SSOTs: found {} empirical test probe citations",
            total_probes
        );
    }
    Ok(total_probes)
}
