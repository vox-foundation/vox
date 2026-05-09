//! `cargo run -p vox-rule-pack --example load_rules -- <path>`
//!
//! Loads a rule pack YAML and prints rule count + each rule id.
//! Used in CI smoke tests; not part of the public surface.

use std::path::PathBuf;
use vox_rule_pack::RulePack;

fn main() -> anyhow::Result<()> {
    let path: PathBuf = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: load_rules <path>"))?
        .into();
    let pack = RulePack::load_from_path(&path)?;
    println!("loaded {} rules from {}", pack.len(), path.display());
    for rule in pack.rules() {
        println!(
            "  - {}  [{:?}, {} lang(s)]",
            rule.id,
            rule.severity,
            rule.languages.len()
        );
    }
    Ok(())
}
