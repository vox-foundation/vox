//! Parity: `mens/config/lora_scratch_arch.yaml` vs `LoraVoxTransformer` constants in `lora/part_vox.rs`.
//! Integration test so it runs without enabling the `mens` feature on the library crate.

#[test]
fn lora_scratch_arch_yaml_matches_rust_constants_documented_in_part_vox() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mens/config/lora_scratch_arch.yaml");
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    fn yaml_u64(raw: &str, key: &str) -> u64 {
        for line in raw.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix(key) {
                let rest = rest.trim_start();
                if let Some(v) = rest.strip_prefix(':') {
                    return v
                        .trim()
                        .parse()
                        .unwrap_or_else(|_| panic!("bad int for {key}"));
                }
            }
        }
        panic!("missing key {key}");
    }

    assert_eq!(yaml_u64(&raw, "max_seq_len"), 512);
    assert_eq!(yaml_u64(&raw, "d_model"), 512);
    assert_eq!(yaml_u64(&raw, "n_heads"), 8);
    assert_eq!(yaml_u64(&raw, "n_layers"), 6);
}
