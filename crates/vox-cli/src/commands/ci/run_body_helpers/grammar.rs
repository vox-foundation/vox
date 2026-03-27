use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;
use crate::commands::ci::cmd_enums::GrammarDriftEmit;

use super::hash::sha256_hex_lower;

pub(crate) fn run_grammar_drift(root: &Path, emit: Option<GrammarDriftEmit>) -> Result<()> {
    let prompt = crate::training::generate_system_prompt();
    let fingerprint = sha256_hex_lower(prompt.as_bytes());
    let path_mens = root.join("mens/data/grammar_fingerprint.txt");
    let path_populi = root.join("populi/data/grammar_fingerprint.txt");
    let read_one = |p: &Path| -> String {
        if !p.is_file() {
            return String::new();
        }
        read_utf8_path_capped(p)
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let stored_mens = read_one(&path_mens);
    let stored_populi = read_one(&path_populi);
    let stored = if !stored_mens.is_empty() {
        stored_mens
    } else {
        stored_populi
    };
    let drift = fingerprint != stored;
    if drift {
        let content = format!("{fingerprint}\n");
        for path in [&path_mens, &path_populi] {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, &content)?;
        }
        eprintln!(
            "Grammar drift detected (fingerprint changed). Updated {} and {}.",
            path_mens.display(),
            path_populi.display()
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
