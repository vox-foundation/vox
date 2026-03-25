//! CLI surface for `vox mens oratio` (speech-to-text).

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Subcommands for Oratio (STT / transcripts).
#[derive(Parser, Debug)]
pub enum OratioAction {
    /// Transcribe a file to text (native STT when enabled; `.txt`/`.md` fixtures always)
    Transcribe {
        /// Audio or transcript fixture path
        path: PathBuf,
        /// Print JSON instead of plain text
        #[arg(long, default_value = "false")]
        json: bool,
        /// Emit refined text when available (default: yes)
        #[arg(long, default_value = "true")]
        refined: bool,
    },
    /// Show which Oratio backends and passthrough modes are available
    Status,
}

/// Run `vox mens oratio …`.
pub fn run(action: OratioAction, global_json: bool) -> Result<()> {
    match action {
        OratioAction::Transcribe {
            path,
            json,
            refined,
        } => {
            let use_json = json || global_json;
            let t = vox_oratio::transcribe_path(&path)?;
            let text = if refined {
                t.display_text().to_string()
            } else {
                t.raw_text.clone()
            };
            if use_json {
                let payload = serde_json::json!({
                    "path": path,
                    "raw_text": t.raw_text,
                    "refined_text": t.refined_text,
                    "text": text,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("{text}");
            }
            Ok(())
        }
        OratioAction::Status => {
            println!("{}", vox_oratio::transcript_status());
            println!(
                "{}",
                serde_json::to_string_pretty(&vox_oratio::candle_backend_status_json())?
            );
            Ok(())
        }
    }
}
