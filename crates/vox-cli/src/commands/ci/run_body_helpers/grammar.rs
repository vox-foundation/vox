use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::commands::ci::cmd_enums::GrammarDriftEmit;

use super::hash::sha256_hex_lower;

pub(crate) fn run_grammar_drift(root: &Path, emit: Option<GrammarDriftEmit>) -> Result<()> {
    let prompt = crate::training::generate_system_prompt();
    let fingerprint = sha256_hex_lower(prompt.as_bytes());
    let path = root.join("mens/data/grammar_fingerprint.txt");
    let stored = if path.is_file() {
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        String::new()
    };
    let drift = fingerprint != stored;
    if drift {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, format!("{fingerprint}\n"))?;
        eprintln!(
            "Grammar drift detected (fingerprint changed). Updated {}.",
            path.display()
        );
    } else {
        eprintln!("No grammar drift detected.");
    }
    let drift_line = if drift { "drift=true" } else { "drift=false" };
    match emit {
        Some(GrammarDriftEmit::Github) => println!("{drift_line}"),
        Some(GrammarDriftEmit::Gitlab) => {
            let p = root.join("drift.env");
            fs::write(&p, format!("{drift_line}\n"))?;
            eprintln!("Wrote {}", p.display());
        }
        None => {}
    }
    Ok(())
}
