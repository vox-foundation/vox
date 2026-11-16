//! Zero-download end-to-end smoke test for synthetic untied `Qwen3.8` (`qwen3_5` VLM):
//! sharded checkpoint -> Metal QLoRA train (rsLoRA + token-norm + packed docs) ->
//! adapter merge -> `InferenceEngine::load` -> prompt prefill + incremental cached decode.

use candle_core::{Device, Tensor};
use std::collections::{BTreeMap, HashMap};
use vox_plugin_mens_candle_metal::candle_qlora_train::run_candle_qlora_train;
use vox_plugin_mens_candle_metal::config::{ChatmlConfig, LoraTrainingConfig, MensTokenizerMode};
use vox_plugin_mens_candle_metal::device::DeviceKind;
use vox_plugin_mens_candle_metal::inference::InferenceEngine;
use vox_plugin_mens_candle_metal::merge::merge_qlora_adapter;

fn det_tensor(shape: &[usize], seed: usize, dev: &Device) -> Tensor {
    let count: usize = shape.iter().product();
    let data: Vec<f32> = (0..count)
        .map(|i| 0.02 * ((((i.wrapping_mul(17) + seed) % 29) as f32) - 14.0))
        .collect();
    Tensor::from_vec(data, shape, dev).expect("tensor")
}

fn synthetic_tokenizer_json() -> String {
    let tokens = [
        "<unk>",
        "<|im_start|>",
        "<|im_end|>",
        "<|endoftext|>",
        "system",
        "user",
        "assistant",
        "<think>",
        "</think>",
        "You",
        "are",
        "Vox.",
        "write",
        "fn",
        "add",
        "sub",
        "mul",
        "div",
        "a",
        "b",
        "x",
        "y",
        "ret",
    ];
    let mut vocab = serde_json::Map::new();
    for (i, tok) in tokens.iter().enumerate() {
        vocab.insert((*tok).to_string(), serde_json::json!(i));
    }
    let added_tokens: Vec<serde_json::Value> = [
        ("<|im_start|>", 1usize, true),
        ("<|im_end|>", 2, true),
        ("<|endoftext|>", 3, true),
        ("<think>", 7, false),
        ("</think>", 8, false),
    ]
    .into_iter()
    .map(|(content, id, special)| {
        serde_json::json!({
            "id": id,
            "content": content,
            "single_word": false,
            "lstrip": false,
            "rstrip": false,
            "normalized": false,
            "special": special,
        })
    })
    .collect();

    serde_json::json!({
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": added_tokens,
        "normalizer": null,
        "pre_tokenizer": {"type": "Whitespace"},
        "post_processor": null,
        "decoder": null,
        "model": {
            "type": "WordLevel",
            "vocab": vocab,
            "unk_token": "<unk>"
        }
    })
    .to_string()
}

use std::path::{Path, PathBuf};

fn build_synthetic_qwen35_base(
    base_dir: &Path,
    eos_token_id: u32,
    write_generation_config: bool,
) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let dev = Device::Cpu;
    std::fs::create_dir_all(base_dir).expect("mkdir base");

    let cfg_json = serde_json::json!({
        "architectures": ["Qwen3_5ForConditionalGeneration"],
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
            "eos_token_id": eos_token_id
        }
    });
    let cfg_path = base_dir.join("config.json");
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&cfg_json).unwrap()).expect("write config");

    if write_generation_config {
        let gen_cfg_path = base_dir.join("generation_config.json");
        std::fs::write(&gen_cfg_path, r#"{"eos_token_id": [2, 3]}"#)
            .expect("write generation_config");
    }

    let tok_path = base_dir.join("tokenizer.json");
    std::fs::write(&tok_path, synthetic_tokenizer_json()).expect("write tokenizer");

    let mut shard1: HashMap<String, Tensor> = HashMap::new();
    let mut shard2: HashMap<String, Tensor> = HashMap::new();

    shard1.insert(
        "model.language_model.embed_tokens.weight".into(),
        det_tensor(&[128, 256], 1, &dev),
    );
    let p0 = "model.language_model.layers.0";
    shard1.insert(
        format!("{p0}.input_layernorm.weight"),
        Tensor::zeros((256,), candle_core::DType::F32, &dev).unwrap(),
    );
    shard1.insert(
        format!("{p0}.post_attention_layernorm.weight"),
        Tensor::zeros((256,), candle_core::DType::F32, &dev).unwrap(),
    );
    for (k, shape, s) in [
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
        (format!("{p0}.mlp.gate_proj.weight"), vec![512, 256], 13),
        (format!("{p0}.mlp.up_proj.weight"), vec![512, 256], 14),
        (format!("{p0}.mlp.down_proj.weight"), vec![256, 512], 15),
    ] {
        shard1.insert(k, det_tensor(&shape, s, &dev));
    }
    shard1.insert(
        format!("{p0}.linear_attn.dt_bias"),
        Tensor::zeros((4,), candle_core::DType::F32, &dev).unwrap(),
    );
    shard1.insert(
        format!("{p0}.linear_attn.A_log"),
        Tensor::full(-2.0f32, (4,), &dev).unwrap(),
    );
    shard1.insert(
        format!("{p0}.linear_attn.norm.weight"),
        Tensor::ones((64,), candle_core::DType::F32, &dev).unwrap(),
    );

    let p1 = "model.language_model.layers.1";
    shard2.insert(
        format!("{p1}.input_layernorm.weight"),
        Tensor::zeros((256,), candle_core::DType::F32, &dev).unwrap(),
    );
    shard2.insert(
        format!("{p1}.post_attention_layernorm.weight"),
        Tensor::zeros((256,), candle_core::DType::F32, &dev).unwrap(),
    );
    shard2.insert(
        format!("{p1}.self_attn.q_norm.weight"),
        Tensor::zeros((64,), candle_core::DType::F32, &dev).unwrap(),
    );
    shard2.insert(
        format!("{p1}.self_attn.k_norm.weight"),
        Tensor::zeros((64,), candle_core::DType::F32, &dev).unwrap(),
    );
    for (k, shape, s) in [
        (format!("{p1}.self_attn.q_proj.weight"), vec![512, 256], 18),
        (format!("{p1}.self_attn.k_proj.weight"), vec![128, 256], 19),
        (format!("{p1}.self_attn.v_proj.weight"), vec![128, 256], 20),
        (format!("{p1}.self_attn.o_proj.weight"), vec![256, 256], 21),
        (format!("{p1}.mlp.gate_proj.weight"), vec![512, 256], 24),
        (format!("{p1}.mlp.up_proj.weight"), vec![512, 256], 25),
        (format!("{p1}.mlp.down_proj.weight"), vec![256, 512], 26),
    ] {
        shard2.insert(k, det_tensor(&shape, s, &dev));
    }
    shard2.insert(
        "model.language_model.norm.weight".into(),
        Tensor::zeros((256,), candle_core::DType::F32, &dev).unwrap(),
    );
    shard2.insert("lm_head.weight".into(), det_tensor(&[128, 256], 28, &dev));

    let shard1_path = base_dir.join("model-00001-of-00002.safetensors");
    let shard2_path = base_dir.join("model-00002-of-00002.safetensors");
    candle_core::safetensors::save(&shard1, &shard1_path).expect("save shard 1");
    candle_core::safetensors::save(&shard2, &shard2_path).expect("save shard 2");

    let mut weight_map = BTreeMap::new();
    for k in shard1.keys() {
        weight_map.insert(k.clone(), "model-00001-of-00002.safetensors");
    }
    for k in shard2.keys() {
        weight_map.insert(k.clone(), "model-00002-of-00002.safetensors");
    }
    std::fs::write(
        base_dir.join("model.safetensors.index.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "metadata": {"total_size": 0},
            "weight_map": weight_map,
        }))
        .unwrap(),
    )
    .expect("write index");

    (shard1_path, shard2_path, cfg_path, tok_path)
}

#[test]
fn qwen38_vlm_metal_train_merge_load_and_cached_decode_smoke() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let base_dir = tmp.path().join("qwen38_base");
    let data_dir = tmp.path().join("data");
    let run_dir = tmp.path().join("run_out");
    std::fs::create_dir_all(&data_dir).expect("mkdir data");
    std::fs::create_dir_all(&run_dir).expect("mkdir run");

    let (shard1_path, shard2_path, cfg_path, tok_path) =
        build_synthetic_qwen35_base(&base_dir, 2, true);

    let train_path = data_dir.join("train.jsonl");
    let rows = ["add", "sub", "mul", "div"]
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "prompt": format!("write fn {name}"),
                "response": format!("<think> a b </think> fn {name} x y ret x"),
                "rating": 5,
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&train_path, format!("{rows}\n")).expect("write train.jsonl");

    let cfg = LoraTrainingConfig {
        base_model: Some(base_dir.to_string_lossy().into_owned()),
        base_model_paths: Some((vec![shard1_path, shard2_path], cfg_path)),
        tokenizer_path: Some(tok_path),
        train_file: Some(train_path),
        rank: 4,
        alpha: 8.0,
        seq_len: 64,
        batch_size: 1,
        grad_accum: 1,
        epochs: 1,
        learning_rate: 1e-4,
        warmup_steps: 1,
        min_rating: 1,
        validation_split_ratio: Some(0.0),
        tokenizer_mode: MensTokenizerMode::Hf,
        allow_cpu_fallback: true,
        chatml: ChatmlConfig {
            user_no_think_switch: false,
            ..ChatmlConfig::default()
        },
        use_rslora: true,
        token_normalized_grad_accum: true,
        sequence_packing: true,
        checkpoint_every: None,
        force_restart: true,
        ..LoraTrainingConfig::default()
    };

    let summary = run_candle_qlora_train(
        data_dir.as_path(),
        Some(run_dir.as_path()),
        &cfg,
        DeviceKind::Metal,
        "You are Vox.",
    )
    .expect("synthetic Qwen3.8 VLM QLoRA train");
    assert!(summary.total_steps > 0, "expected >= 1 training step");

    let adapter_path = run_dir.join("candle_qlora_adapter.safetensors");
    let manifest_path = run_dir.join("adapter_manifest.json");
    assert!(adapter_path.is_file(), "adapter safetensors missing");
    assert!(manifest_path.is_file(), "adapter manifest missing");

    let merged_path = run_dir.join("merged.safetensors");
    merge_qlora_adapter(
        &base_dir.to_string_lossy(),
        &adapter_path.to_string_lossy(),
        &merged_path.to_string_lossy(),
    )
    .expect("merge qlora adapter");
    assert!(merged_path.is_file(), "merged safetensors missing");

    let merged_tensors = candle_core::safetensors::load(&merged_path, &Device::Cpu)
        .expect("load merged safetensors");
    assert!(
        merged_tensors.contains_key("lm_head.weight"),
        "merged checkpoint must retain untied lm_head.weight"
    );

    assert!(
        run_dir.join("generation_config.json").is_file(),
        "finalize_training_run must copy generation_config.json into run_dir"
    );

    let engine = InferenceEngine::load(run_dir.as_path(), &DeviceKind::Metal)
        .expect("load merged Qwen3.8 engine");
    assert_eq!(engine.eos_token_ids, vec![2, 3]);
    assert!(!engine.user_no_think_switch);

    let mut caches = engine.model.empty_cache();
    assert_eq!(caches.len(), 2);

    let prompt_ids = Tensor::new(&[1u32, 4, 9, 2], &engine.device)
        .and_then(|t| t.unsqueeze(0))
        .expect("prompt_ids");
    let prefill_logits = engine
        .model
        .forward_with_cache(&prompt_ids, 0, Some(&mut caches))
        .expect("prefill");
    assert_eq!(prefill_logits.dims(), &[1, 4, 128]);
    let prefill_vals = prefill_logits
        .to_dtype(candle_core::DType::F32)
        .expect("f32 cast")
        .flatten_all()
        .expect("flatten prefill")
        .to_vec1::<f32>()
        .expect("prefill vec");
    assert!(
        prefill_vals.iter().all(|v| v.is_finite()),
        "prefill logits must be finite"
    );

    for (pos, tok) in [(4usize, 13u32), (5usize, 14u32)] {
        let step_id = Tensor::new(&[tok], &engine.device)
            .and_then(|t| t.unsqueeze(0))
            .expect("step_id");
        let step_logits = engine
            .model
            .forward_with_cache(&step_id, pos, Some(&mut caches))
            .expect("cached decode step");
        assert_eq!(step_logits.dims(), &[1, 1, 128]);
        let vals = step_logits
            .to_dtype(candle_core::DType::F32)
            .expect("f32 cast")
            .flatten_all()
            .expect("flatten decode")
            .to_vec1::<f32>()
            .expect("decode vec");
        assert!(
            vals.iter().all(|v| v.is_finite()),
            "cached decode logits at pos={pos} must be finite"
        );
    }
}

#[test]
fn qwen35_without_generation_config_includes_im_end_eos() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let base_dir = tmp.path().join("qwen35_no_gen_cfg");
    build_synthetic_qwen35_base(&base_dir, 3, false);

    assert!(!base_dir.join("generation_config.json").is_file());
    let engine =
        InferenceEngine::load(base_dir.as_path(), &DeviceKind::Cpu).expect("load base engine");
    assert_eq!(engine.eos_token_ids, vec![3, 2]);
}

#[test]
fn qwen38_vlm_max_runtime_secs_caps_cleanly() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let base_dir = tmp.path().join("qwen38_base_cap");
    let data_dir = tmp.path().join("data_cap");
    let run_dir = tmp.path().join("run_cap");
    std::fs::create_dir_all(&data_dir).expect("mkdir data");
    std::fs::create_dir_all(&run_dir).expect("mkdir run");

    let (shard1_path, shard2_path, cfg_path, tok_path) =
        build_synthetic_qwen35_base(&base_dir, 2, true);

    let train_path = data_dir.join("train.jsonl");
    let rows = ["add", "sub", "mul", "div"]
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "prompt": format!("write fn {name}"),
                "response": format!("<think> a b </think> fn {name} x y ret x"),
                "rating": 5,
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&train_path, format!("{rows}\n")).expect("write train.jsonl");

    let cfg = LoraTrainingConfig {
        base_model: Some(base_dir.to_string_lossy().into_owned()),
        base_model_paths: Some((vec![shard1_path, shard2_path], cfg_path)),
        tokenizer_path: Some(tok_path),
        train_file: Some(train_path),
        rank: 4,
        alpha: 8.0,
        seq_len: 64,
        batch_size: 1,
        grad_accum: 1,
        epochs: 10,
        learning_rate: 1e-4,
        warmup_steps: 1,
        min_rating: 1,
        validation_split_ratio: Some(0.0),
        tokenizer_mode: MensTokenizerMode::Hf,
        allow_cpu_fallback: true,
        chatml: ChatmlConfig {
            user_no_think_switch: false,
            ..ChatmlConfig::default()
        },
        use_rslora: true,
        token_normalized_grad_accum: true,
        sequence_packing: false,
        checkpoint_every: None,
        force_restart: true,
        max_runtime_secs: Some(0),
        ..LoraTrainingConfig::default()
    };

    let summary = run_candle_qlora_train(
        data_dir.as_path(),
        Some(run_dir.as_path()),
        &cfg,
        DeviceKind::Cpu,
        "You are Vox.",
    )
    .expect("capped QLoRA train");
    assert_eq!(
        summary.total_steps, 1,
        "should stop after 1st optimizer step"
    );

    let raw = std::fs::read_to_string(run_dir.join("run_summary.json")).expect("read run_summary");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse run_summary");
    assert_eq!(
        v["capped"].as_bool(),
        Some(true),
        "expected capped=true: {v}"
    );
    assert_eq!(
        v["optimizer_step_count"].as_u64(),
        Some(1),
        "expected optimizer_step_count=1: {v}"
    );
}
