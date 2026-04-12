//! `vox grammar export` — emit the Vox language grammar in multiple formats.

use clap::Parser;
use std::path::PathBuf;
use vox_grammar_export::{GrammarExportConfig, GrammarFormat};

/// Export the Vox language grammar for MENS training or constrained inference.
#[derive(Parser, Debug)]
pub struct GrammarParams {
    /// Format to export: ebnf | gbnf | lark | json-schema.
    #[arg(short, long, default_value = "ebnf")]
    format: String,
    /// Optional path to write the grammar output (defaults to stdout).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

pub fn handle(params: GrammarParams) {
    let format = match params.format.to_lowercase().as_str() {
        "ebnf" => GrammarFormat::Ebnf,
        "gbnf" => GrammarFormat::Gbnf,
        "lark" => GrammarFormat::Lark,
        "json-schema" | "json_schema" | "jsonschema" => GrammarFormat::JsonSchema,
        other => {
            eprintln!(
                "Unknown grammar format: '{other}'. Valid formats: ebnf, gbnf, lark, json-schema."
            );
            std::process::exit(1);
        }
    };

    let config = GrammarExportConfig {
        format,
        ..GrammarExportConfig::default()
    };

    let result = vox_grammar_export::export(&config);

    match params.output {
        None => println!("{}", result.grammar_text),
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .unwrap_or_else(|e| eprintln!("Warning: could not create dirs: {e}"));
                }
            }
            std::fs::write(&path, result.grammar_text.as_bytes()).unwrap_or_else(|e| {
                eprintln!("Failed to write grammar to {}: {e}", path.display());
                std::process::exit(1);
            });
            eprintln!(
                "Exported {} grammar ({} rules) to {}",
                config.format.as_str(),
                result.rule_count,
                path.display()
            );
        }
    }
}
