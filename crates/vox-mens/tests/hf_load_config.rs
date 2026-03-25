//! Integration tests: HF `config.json` parsing (`tensor::hf_load`).

use std::path::PathBuf;

use tempfile::tempdir;
use vox_mens::tensor::hf_load::{
    ConfigDims, HfArchitecture, HfTransformerLayout, config_dims_for_architecture,
    detect_hf_architecture, parse_transformer_layout,
};

fn write_config(dir: &std::path::Path, name: &str, json: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, json).expect("write config");
    p
}

#[test]
fn detect_gpt2_from_minimal_config() {
    let dir = tempdir().expect("tempdir");
    let p = write_config(
        dir.path(),
        "config.json",
        r#"{"n_embd":32,"n_head":4,"n_layer":2,"vocab_size":100}"#,
    );
    let arch = detect_hf_architecture(&p).expect("detect");
    assert_eq!(arch, HfArchitecture::Gpt2);
    let d = config_dims_for_architecture(&p, arch).expect("dims");
    assert_eq!(
        d,
        ConfigDims {
            n_embd: 32,
            n_head: 4,
            n_layer: 2,
            vocab_size: 100,
        }
    );
}

#[test]
fn parse_layout_from_fixture_gpt2() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = root.join("tests/fixtures/hf/gpt2_tiny_config.json");
    let layout = parse_transformer_layout(&p).expect("layout");
    assert_eq!(layout.architecture, HfArchitecture::Gpt2);
    assert_eq!(layout.dims.n_embd, 32);
    assert_eq!(layout.dims.n_layer, 2);
}

#[test]
fn parse_layout_from_fixture_llama_maps_to_stacked_family() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = root.join("tests/fixtures/hf/llama_tiny_config.json");
    let layout = parse_transformer_layout(&p).expect("layout");
    assert_eq!(layout.architecture, HfArchitecture::Qwen2);
    assert_eq!(layout.dims.n_embd, 64);
    assert_eq!(layout.dims.n_layer, 3);
    assert!(layout.primary_architecture.contains("Llama"));
}

#[test]
fn detect_qwen2_from_model_type() {
    let dir = tempdir().expect("tempdir");
    let p = write_config(
        dir.path(),
        "config.json",
        r#"{"model_type":"qwen2","hidden_size":64,"num_attention_heads":8,"num_hidden_layers":3,"vocab_size":500}"#,
    );
    let arch = detect_hf_architecture(&p).expect("detect");
    assert_eq!(arch, HfArchitecture::Qwen2);
    let d = config_dims_for_architecture(&p, arch).expect("dims");
    assert_eq!(
        d,
        ConfigDims {
            n_embd: 64,
            n_head: 8,
            n_layer: 3,
            vocab_size: 500,
        }
    );
}

#[test]
fn detect_qwen2_5_from_model_type_maps_like_qwen2() {
    let dir = tempdir().expect("tempdir");
    let p = write_config(
        dir.path(),
        "config.json",
        r#"{"model_type":"qwen2.5","hidden_size":64,"num_attention_heads":8,"num_hidden_layers":2,"vocab_size":400}"#,
    );
    let arch = detect_hf_architecture(&p).expect("detect");
    assert_eq!(arch, HfArchitecture::Qwen2);
}

#[test]
fn hf_transformer_layout_llama_class_json_fixture() {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hf_config_llama_class.json");
    let l = HfTransformerLayout::from_config_path(&p).expect("layout");
    assert_eq!(l.num_hidden_layers, 32);
    assert_eq!(l.hidden_size, 4096);
}

#[test]
fn hf_transformer_layout_mistral_json_fixture() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hf_config_mistral_like.json");
    let l = HfTransformerLayout::from_config_path(&p).expect("layout");
    assert_eq!(l.model_type, "mistral");
    let arch = detect_hf_architecture(&p).expect("arch");
    assert_eq!(arch, HfArchitecture::Qwen2);
}
