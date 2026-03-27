#![cfg(feature = "mens-candle-qlora")]

use vox_populi::mens::tensor::adapter_schema_v3::PopuliAdapterManifestV3;
use vox_populi::mens::tensor::candle_qlora_merge::QloraAdapterMetaV2;
use vox_populi::mens::tensor::finetune_contract::{AdapterMethod, BaseQuantMode};
use vox_populi::mens::tensor::hf_load::{HfArchitecture, HfTransformerLayout};

#[test]
fn qwen35_layout_parses_linear_geometry_fields() {
    let cfg = r#"{
      "model_type":"qwen3_5",
      "text_config":{
        "hidden_size":8,
        "num_attention_heads":1,
        "num_hidden_layers":1,
        "num_key_value_heads":1,
        "vocab_size":16,
        "layer_types":["linear_attention"],
        "head_dim":8,
        "linear_num_key_heads":1,
        "linear_num_value_heads":1,
        "linear_key_head_dim":2,
        "linear_value_head_dim":2,
        "linear_conv_kernel_dim":4,
        "rope_parameters":{"rope_theta":10000,"partial_rotary_factor":0.25}
      }
    }"#;
    let layout = HfTransformerLayout::from_config_json_str(cfg).expect("layout");
    assert_eq!(layout.architecture, HfArchitecture::Qwen35);
    assert_eq!(layout.linear_num_key_heads, Some(1));
    assert_eq!(layout.linear_num_value_heads, Some(1));
    assert_eq!(layout.linear_key_head_dim, Some(2));
    assert_eq!(layout.linear_value_head_dim, Some(2));
    assert_eq!(layout.linear_conv_kernel_dim, Some(4));
    assert_eq!(layout.rope_partial_rotary_factor, Some(0.25));
}

#[test]
fn qwen35_manifest_v3_roundtrip_preserves_base_model_for_merge() {
    let mut base_key_map = std::collections::HashMap::new();
    base_key_map.insert("lm_head".to_string(), "model.language_model.embed_tokens.weight".to_string());
    let v3 = PopuliAdapterManifestV3::new(
        AdapterMethod::Qlora,
        BaseQuantMode::Nf4,
        true,
        base_key_map,
        vec!["lm_head".into()],
        1024,
        256,
        16,
        32,
        Some("Qwen/Qwen3.5-4B".into()),
        None,
    );
    let v2 = vox_populi::mens::tensor::adapter_schema_v3::to_qlora_meta_v2_for_merge(&v3)
        .expect("v3->v2 bridge");
    assert_eq!(v2.base_model.as_deref(), Some("Qwen/Qwen3.5-4B"));
    let back = vox_populi::mens::tensor::adapter_schema_v3::from_qlora_meta_v2(&QloraAdapterMetaV2 {
        base_model: Some("Qwen/Qwen3.5-4B".into()),
        ..v2
    });
    assert_eq!(back.base_model.as_deref(), Some("Qwen/Qwen3.5-4B"));
}
