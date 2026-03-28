//! Generate and verify [`doc-inventory.json`](../../docs/agents/doc-inventory.json) (schema v3).
//!
//! Replaces the retired Python inventory scripts; verify via `vox ci doc-inventory verify`.

mod bounded_fs;
mod cli_generate;
mod constants;
mod counts;
mod file_entry;
mod hints;
mod inventory_gen;
mod relevance;
mod types;
mod verify_normalize;
mod walk;

pub use cli_generate::run_generate_inventory_cli;
pub use constants::DEFAULT_INVENTORY_PATH;
pub use inventory_gen::generate;
pub use relevance::relevance_score;
pub use types::{DocInventory, FileEntry, SymbolHint, SymbolHintGroup};
pub use verify_normalize::{strip_generated_at, verify_fresh};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relevance_score_prefers_hotspot_at_same_doc_density() {
        let tier0 = FileEntry {
            path: "crates/example/src/lib.rs".into(),
            kind: "rust".into(),
            lines_total: 200,
            lines_triple_slash: 12,
            lines_inner_doc: 0,
            lines_plain_comment: 4,
            lines_other_doc_signal: 0,
            hotspot_tier: 0,
            notes: String::new(),
        };
        let tier1 = FileEntry {
            hotspot_tier: 1,
            ..tier0.clone()
        };
        assert!(relevance_score(&tier1) > relevance_score(&tier0));
    }

    #[test]
    fn strip_generated_at_removes_field() {
        let v = serde_json::json!({"schema_version":3,"generated_at":"x","files":[]});
        let s = strip_generated_at(v);
        assert!(s.get("generated_at").is_none());
    }

    #[test]
    fn normalize_json_sorts_object_keys() {
        let v = serde_json::json!({"z":1,"a":{"y":2,"b":3}});
        let n = verify_normalize::normalize_json_value(v);
        let obj = n.as_object().expect("object");
        let keys: Vec<_> = obj.keys().collect();
        assert_eq!(keys, vec!["a", "z"]);
    }

    #[test]
    fn symbol_hints_link_doc_to_next_item() {
        let src = "/// Example doc\nfn sample_fn() -> i32 { 0 }\n";
        let h = hints::rust_symbol_hints(src);
        assert!(
            h.iter().any(|x| x.item_preview.contains("sample_fn")),
            "expected symbol hint for fn after ///, got {h:?}"
        );
    }
}
