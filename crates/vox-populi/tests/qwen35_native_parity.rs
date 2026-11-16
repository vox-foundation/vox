#![cfg(feature = "mens-candle-qlora")]

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
fn qwen38_vlm_sharded_quantize_and_untied_recombine_smoke() {
    use candle_core::{DType, Device, Tensor};
    use std::collections::{BTreeMap, HashMap};
    use vox_quantize::read::SafeTensorsSource;
    use vox_quantize::{DevicePref, QuantMixture, QuantizeRequest, quantize};

    fn det_tensor(shape: &[usize], seed: usize, dev: &Device) -> Tensor {
        let count: usize = shape.iter().product();
        let data: Vec<f32> = (0..count)
            .map(|i| 0.02 * ((((i.wrapping_mul(17) + seed) % 29) as f32) - 14.0))
            .collect();
        Tensor::from_vec(data, shape, dev).expect("tensor")
    }

    let dev = Device::Cpu;
    let tmp = tempfile::tempdir().expect("tempdir");

    // ── Subtest A: Sharded VLM quantize skips unreferenced vision/MTP shard cleanly ──
    let vlm_dir = tmp.path().join("vlm_sharded_base");
    let quant_dir = tmp.path().join("vlm_quant_out");
    std::fs::create_dir_all(&vlm_dir).expect("mkdir vlm_dir");

    let cfg_json = r#"{
      "model_type": "qwen3_5",
      "tie_word_embeddings": false,
      "text_config": {
        "hidden_size": 256,
        "intermediate_size": 512,
        "num_hidden_layers": 2,
        "num_attention_heads": 4,
        "num_key_value_heads": 2,
        "head_dim": 64,
        "vocab_size": 128,
        "tie_word_embeddings": false,
        "rms_norm_eps": 1e-6,
        "layer_types": ["linear_attention", "full_attention"],
        "linear_num_key_heads": 2,
        "linear_num_value_heads": 4,
        "linear_key_head_dim": 64,
        "linear_value_head_dim": 64,
        "linear_conv_kernel_dim": 4,
        "rope_parameters": {"rope_theta": 10000.0, "partial_rotary_factor": 0.25},
        "eos_token_id": 2
      }
    }"#;
    std::fs::write(vlm_dir.join("config.json"), cfg_json).expect("write config");

    let mut shard1: HashMap<String, Tensor> = HashMap::new();
    let mut shard2: HashMap<String, Tensor> = HashMap::new();

    shard1.insert(
        "model.language_model.embed_tokens.weight".into(),
        det_tensor(&[128, 256], 1, &dev),
    );
    let p0 = "model.language_model.layers.0";
    for (k, shape, s) in [
        (format!("{p0}.input_layernorm.weight"), vec![256], 2),
        (
            format!("{p0}.post_attention_layernorm.weight"),
            vec![256],
            3,
        ),
        (
            format!("{p0}.linear_attn.in_proj_qkv.weight"),
            vec![512, 256],
            4,
        ),
        (
            format!("{p0}.linear_attn.in_proj_z.weight"),
            vec![256, 256],
            5,
        ),
        (
            format!("{p0}.linear_attn.in_proj_b.weight"),
            vec![4, 256],
            6,
        ),
        (
            format!("{p0}.linear_attn.in_proj_a.weight"),
            vec![4, 256],
            7,
        ),
        (
            format!("{p0}.linear_attn.out_proj.weight"),
            vec![256, 256],
            8,
        ),
        (
            format!("{p0}.linear_attn.conv1d.weight"),
            vec![512, 1, 4],
            9,
        ),
        (format!("{p0}.linear_attn.dt_bias"), vec![4], 10),
        (format!("{p0}.linear_attn.A_log"), vec![4], 11),
        (format!("{p0}.linear_attn.norm.weight"), vec![64], 12),
        (format!("{p0}.mlp.gate_proj.weight"), vec![512, 256], 13),
        (format!("{p0}.mlp.up_proj.weight"), vec![512, 256], 14),
        (format!("{p0}.mlp.down_proj.weight"), vec![256, 512], 15),
    ] {
        shard1.insert(k, det_tensor(&shape, s, &dev));
    }

    let p1 = "model.language_model.layers.1";
    for (k, shape, s) in [
        (format!("{p1}.input_layernorm.weight"), vec![256], 16),
        (
            format!("{p1}.post_attention_layernorm.weight"),
            vec![256],
            17,
        ),
        (format!("{p1}.self_attn.q_proj.weight"), vec![512, 256], 18),
        (format!("{p1}.self_attn.k_proj.weight"), vec![128, 256], 19),
        (format!("{p1}.self_attn.v_proj.weight"), vec![128, 256], 20),
        (format!("{p1}.self_attn.o_proj.weight"), vec![256, 256], 21),
        (format!("{p1}.self_attn.q_norm.weight"), vec![64], 22),
        (format!("{p1}.self_attn.k_norm.weight"), vec![64], 23),
        (format!("{p1}.mlp.gate_proj.weight"), vec![512, 256], 24),
        (format!("{p1}.mlp.up_proj.weight"), vec![512, 256], 25),
        (format!("{p1}.mlp.down_proj.weight"), vec![256, 512], 26),
    ] {
        shard2.insert(k, det_tensor(&shape, s, &dev));
    }
    shard2.insert(
        "model.language_model.norm.weight".into(),
        det_tensor(&[256], 27, &dev),
    );
    shard2.insert("lm_head.weight".into(), det_tensor(&[128, 256], 28, &dev));

    candle_core::safetensors::save(&shard1, vlm_dir.join("model-00001-of-00003.safetensors"))
        .expect("save shard 1");
    candle_core::safetensors::save(&shard2, vlm_dir.join("model-00002-of-00003.safetensors"))
        .expect("save shard 2");

    // Deliberately point vision/MTP keys at a non-existent shard 3.
    let mut weight_map = BTreeMap::new();
    for k in shard1.keys() {
        weight_map.insert(k.clone(), "model-00001-of-00003.safetensors");
    }
    for k in shard2.keys() {
        weight_map.insert(k.clone(), "model-00002-of-00003.safetensors");
    }
    weight_map.insert(
        "model.visual.blocks.0.attn.proj.weight".into(),
        "model-00003-of-00003.safetensors",
    );
    weight_map.insert(
        "mtp.layers.0.proj.weight".into(),
        "model-00003-of-00003.safetensors",
    );
    let idx_json = serde_json::json!({
        "metadata": { "total_size": 0 },
        "weight_map": weight_map,
    });
    std::fs::write(
        vlm_dir.join("model.safetensors.index.json"),
        serde_json::to_vec_pretty(&idx_json).unwrap(),
    )
    .expect("write index");

    let report = quantize(&QuantizeRequest {
        input_dir: vlm_dir.clone(),
        output_dir: quant_dir.clone(),
        mixture: QuantMixture::Q4KM,
        verify: true,
        device: DevicePref::Cpu,
    })
    .expect("quantize must skip vision/mtp keys mapped to absent shard 3");
    assert!(report.worst_mse.is_finite());

    let meta_raw =
        std::fs::read_to_string(quant_dir.join("quant-metadata.json")).expect("read quant-meta");
    let meta: serde_json::Value = serde_json::from_str(&meta_raw).expect("parse quant-meta");
    let tensors = meta["tensors"].as_object().expect("tensors map");
    assert!(
        !tensors
            .keys()
            .any(|k| k.starts_with("model.visual.") || k.starts_with("mtp.")),
        "vision and MTP tensors must be excluded from quant-metadata.json"
    );
    for f32_key in [
        format!("{p0}.linear_attn.in_proj_a.weight"),
        format!("{p0}.linear_attn.in_proj_b.weight"),
        format!("{p0}.linear_attn.conv1d.weight"),
        format!("{p0}.linear_attn.norm.weight"),
        format!("{p1}.self_attn.q_norm.weight"),
        format!("{p1}.self_attn.k_norm.weight"),
        format!("{p1}.input_layernorm.weight"),
        "model.language_model.norm.weight".to_string(),
    ] {
        let entry = &tensors[&f32_key];
        assert_eq!(entry["quantized"], false, "{f32_key} must stay unquantized");
        assert_eq!(entry["ggml_dtype"], "F32", "{f32_key} must stay F32");
    }
    for quant_key in [
        format!("{p0}.linear_attn.in_proj_qkv.weight"),
        format!("{p1}.self_attn.q_proj.weight"),
        "lm_head.weight".to_string(),
    ] {
        let entry = &tensors[&quant_key];
        assert_eq!(entry["quantized"], true, "{quant_key} must be quantized");
        let dt = entry["ggml_dtype"].as_str().expect("ggml_dtype");
        assert!(
            matches!(dt, "Q4K" | "Q6K"),
            "{quant_key} expected Q4K/Q6K, got {dt}"
        );
    }

    // ── Subtest B: Tied base + synthesized lm_head.weight recombine flips tie_word_embeddings ──
    let tied_base = tmp.path().join("tied_base");
    let recombined = tmp.path().join("recombined");
    std::fs::create_dir_all(&tied_base).expect("mkdir tied_base");

    let mut tied_tensors: HashMap<String, Tensor> = HashMap::new();
    tied_tensors.insert(
        "model.language_model.embed_tokens.weight".into(),
        Tensor::ones((128, 256), DType::F32, &dev).unwrap(),
    );
    tied_tensors.insert(
        "model.language_model.norm.weight".into(),
        Tensor::ones((256,), DType::F32, &dev).unwrap(),
    );
    candle_core::safetensors::save(&tied_tensors, tied_base.join("model.safetensors"))
        .expect("save tied base");
    let tied_cfg = serde_json::json!({
        "model_type": "qwen3_5",
        "tie_word_embeddings": true,
        "text_config": {
            "hidden_size": 256,
            "num_attention_heads": 4,
            "num_hidden_layers": 1,
            "vocab_size": 128,
            "tie_word_embeddings": true
        }
    });
    std::fs::write(
        tied_base.join("config.json"),
        serde_json::to_vec_pretty(&tied_cfg).unwrap(),
    )
    .expect("write tied config");

    let merged_path = tmp.path().join("merged.safetensors");
    let mut merged: HashMap<String, Tensor> = HashMap::new();
    merged.insert(
        "lm_head.weight".into(),
        Tensor::full(2.0f32, (128, 256), &dev).unwrap(),
    );
    candle_core::safetensors::save(&merged, &merged_path).expect("save merged");

    vox_quantize::recombine::recombine(&tied_base, &merged_path, &recombined)
        .expect("recombine with synthesized lm_head.weight must succeed");

    let recombined_cfg_raw =
        std::fs::read_to_string(recombined.join("config.json")).expect("read recombined cfg");
    let recombined_cfg: serde_json::Value =
        serde_json::from_str(&recombined_cfg_raw).expect("parse recombined cfg");
    assert_eq!(recombined_cfg["tie_word_embeddings"], false);
    assert_eq!(recombined_cfg["text_config"]["tie_word_embeddings"], false);

    let recombined_src = SafeTensorsSource::open(&recombined).expect("open recombined source");
    assert!(
        recombined_src
            .tensor_names()
            .contains(&"model.language_model.embed_tokens.weight".to_string())
    );
    assert!(
        recombined_src
            .tensor_names()
            .contains(&"lm_head.weight".to_string())
    );
    assert_eq!(
        recombined_src
            .load_f32("model.language_model.embed_tokens.weight")
            .expect("load embed")
            .dims(),
        &[128, 256]
    );
    assert_eq!(
        recombined_src
            .load_f32("lm_head.weight")
            .expect("load lm_head")
            .dims(),
        &[128, 256]
    );
}
