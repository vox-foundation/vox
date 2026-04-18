use crate::commands::mens::eval_local;
use crate::commands::mens::merge_weights;
use crate::commands::mens::probe;
use crate::commands::mens::status;
use crate::commands::schola::merge_qlora;
use std::path::PathBuf;

#[test]
fn probe_runs_without_gpu() {
    let result = probe::run_probe(false);
    assert!(result.is_ok());
}

#[test]
fn probe_verbose_runs_without_gpu() {
    let result = probe::run_probe(true);
    assert!(result.is_ok());
}

#[test]
fn status_missing_dir_reports_gracefully() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(status::run_status(
        Some(PathBuf::from("/nonexistent/run/dir")),
        false,
        false,
        false,
    ));
    assert!(
        result.is_ok(),
        "missing telemetry should not error: {:?}",
        result
    );
}

#[test]
fn status_json_missing_dir() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime for status test");
    let result = rt.block_on(status::run_status(
        Some(PathBuf::from("/nonexistent/run/dir")),
        true,
        false,
        false,
    ));
    assert!(result.is_ok());
}

#[test]
fn merge_weights_missing_checkpoint_errors() {
    let result = merge_weights::run_merge_weights(
        PathBuf::from("/nonexistent/model.bin"),
        None,
        16,
        32.0,
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not found") || msg.contains("Checkpoint"),
        "expected checkpoint error: {msg}"
    );
}

#[test]
fn merge_weights_rejects_candle_qlora_adapter_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = dir.path().join("candle_qlora_adapter.safetensors");
    std::fs::write(&p, []).expect("touch adapter");
    let result = merge_weights::run_merge_weights(p, None, 8, 16.0);
    assert!(result.is_err(), "expected rejection of Candle adapter path");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Candle") && msg.contains("merge"),
        "expected Candle merge guard: {msg}"
    );
    assert!(
        msg.contains("merge-qlora"),
        "expected pointer to merge-qlora: {msg}"
    );
}

#[test]
fn merge_qlora_rejects_burn_bin_adapter() {
    let dir = tempfile::tempdir().expect("tempdir");
    let adapter = dir.path().join("latest.bin");
    std::fs::write(&adapter, [1u8, 2, 3]).expect("touch bin");
    let meta = dir.path().join("meta.json");
    std::fs::write(&meta, "{}").expect("meta");
    let base = dir.path().join("base.safetensors");
    std::fs::write(&base, []).expect("base shard");
    let out = dir.path().join("merged.safetensors");
    let result = merge_qlora::run_merge_qlora(vec![base], adapter, meta, out);
    assert!(result.is_err(), "expected rejection of Burn bin adapter");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("merge-weights"),
        "expected pointer to merge-weights: {msg}"
    );
    assert!(
        msg.contains("safetensors") || msg.contains("Candle"),
        "expected Candle safetensors hint: {msg}"
    );
}

#[test]
fn merge_qlora_cli_roundtrip_lm_head_subset() {
    use std::collections::HashMap;

    use safetensors::SafeTensors;
    use safetensors::tensor::{Dtype, TensorView};
    use serde_json::json;

    let dir = tempfile::tempdir().expect("tempdir");
    let d = 3usize;
    let vocab = 4usize;
    let rank = 2usize;
    let alpha = 4usize;
    let scale = (alpha as f64 / rank as f64) as f32;

    let w: Vec<f32> = (0..vocab * d).map(|i| i as f32 * 0.1).collect();
    let mut wb = Vec::with_capacity(w.len() * 4);
    for x in &w {
        wb.extend_from_slice(&x.to_le_bytes());
    }
    let mut base_map: HashMap<String, TensorView<'_>> = HashMap::new();
    base_map.insert(
        "wte.weight".into(),
        TensorView::new(Dtype::F32, vec![vocab, d], wb.as_slice()).unwrap(),
    );
    let base_path = dir.path().join("base.safetensors");
    std::fs::write(
        &base_path,
        safetensors::serialize(&base_map, &None).unwrap(),
    )
    .unwrap();

    let fa = vec![1.0f32; rank * d];
    let fb = vec![1.0f32; vocab * rank];
    let mut ab = Vec::new();
    for x in &fa {
        ab.extend_from_slice(&x.to_le_bytes());
    }
    let mut bb = Vec::new();
    for x in &fb {
        bb.extend_from_slice(&x.to_le_bytes());
    }
    let mut ad_map: HashMap<String, TensorView<'_>> = HashMap::new();
    ad_map.insert(
        "lm_head.lora_a".into(),
        TensorView::new(Dtype::F32, vec![rank, d], ab.as_slice()).unwrap(),
    );
    ad_map.insert(
        "lm_head.lora_b".into(),
        TensorView::new(Dtype::F32, vec![vocab, rank], bb.as_slice()).unwrap(),
    );
    let ad_path = dir.path().join("adapter.safetensors");
    std::fs::write(&ad_path, safetensors::serialize(&ad_map, &None).unwrap()).unwrap();

    let meta_path = dir.path().join("meta.json");
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&json!({
            "format": "vox_mens_qlora_lora_only_v2",
            "version": 2,
            "embed_key": "wte.weight",
            "vocab": vocab,
            "d_model": d,
            "rank": rank,
            "alpha": alpha,
            "layer_order": ["lm_head"],
            "base_key_map": { "lm_head": "wte.weight" },
        }))
        .unwrap(),
    )
    .unwrap();

    let out_path = dir.path().join("merged.safetensors");
    merge_qlora::run_merge_qlora(vec![base_path], ad_path, meta_path, out_path.clone())
        .expect("merge-qlora");

    let mut delta = vec![0f32; vocab * d];
    for i in 0..vocab {
        for j in 0..d {
            let mut s = 0f32;
            for k in 0..rank {
                s += fb[i * rank + k] * fa[k * d + j];
            }
            delta[i * d + j] = s * scale;
        }
    }
    let bytes = std::fs::read(&out_path).unwrap();
    let st = SafeTensors::deserialize(&bytes).unwrap();
    let tv = st.tensor("wte.weight").unwrap();
    assert_eq!(tv.dtype(), Dtype::F32);
    let sl = tv.data();
    for i in 0..vocab * d {
        let o = i * 4;
        let got = f32::from_le_bytes([sl[o], sl[o + 1], sl[o + 2], sl[o + 3]]);
        let exp = w[i] + delta[i];
        assert!(
            (got - exp).abs() < 1e-5,
            "idx {i}: expected {exp} got {got}"
        );
    }
}

#[test]
fn merge_qlora_cli_roundtrip_lm_head_subset_adapter_manifest_v3() {
    use std::collections::HashMap;

    use safetensors::SafeTensors;
    use safetensors::tensor::{Dtype, TensorView};
    use vox_populi::mens::tensor::adapter_schema_v3::{
        AdapterProvenanceFields, PopuliAdapterManifestV3,
    };
    use vox_populi::mens::tensor::finetune_contract::{AdapterMethod, BaseQuantMode};

    let dir = tempfile::tempdir().expect("tempdir");
    let d = 3usize;
    let vocab = 4usize;
    let rank = 2usize;
    let alpha = 4usize;
    let scale = (alpha as f64 / rank as f64) as f32;

    let w: Vec<f32> = (0..vocab * d).map(|i| i as f32 * 0.1).collect();
    let mut wb = Vec::with_capacity(w.len() * 4);
    for x in &w {
        wb.extend_from_slice(&x.to_le_bytes());
    }
    let mut base_map: HashMap<String, TensorView<'_>> = HashMap::new();
    base_map.insert(
        "wte.weight".into(),
        TensorView::new(Dtype::F32, vec![vocab, d], wb.as_slice()).unwrap(),
    );
    let base_path = dir.path().join("base.safetensors");
    std::fs::write(
        &base_path,
        safetensors::serialize(&base_map, &None).unwrap(),
    )
    .unwrap();

    let fa = vec![1.0f32; rank * d];
    let fb = vec![1.0f32; vocab * rank];
    let mut ab = Vec::new();
    for x in &fa {
        ab.extend_from_slice(&x.to_le_bytes());
    }
    let mut bb = Vec::new();
    for x in &fb {
        bb.extend_from_slice(&x.to_le_bytes());
    }
    let mut ad_map: HashMap<String, TensorView<'_>> = HashMap::new();
    ad_map.insert(
        "lm_head.lora_a".into(),
        TensorView::new(Dtype::F32, vec![rank, d], ab.as_slice()).unwrap(),
    );
    ad_map.insert(
        "lm_head.lora_b".into(),
        TensorView::new(Dtype::F32, vec![vocab, rank], bb.as_slice()).unwrap(),
    );
    let ad_path = dir.path().join("adapter.safetensors");
    std::fs::write(&ad_path, safetensors::serialize(&ad_map, &None).unwrap()).unwrap();

    let mut base_key_map = HashMap::new();
    base_key_map.insert("lm_head".into(), "wte.weight".into());
    let provenance = AdapterProvenanceFields {
        base_family: Some("kimi-k2.5".into()),
        upstream_model_id: Some("moonshotai/Kimi-K2.5".into()),
        license_class: Some("modified-mit".into()),
        attribution_required: true,
    };
    let v3 = PopuliAdapterManifestV3::new(
        AdapterMethod::Qlora,
        BaseQuantMode::Nf4,
        true,
        base_key_map,
        vec!["lm_head".into()],
        vocab,
        d,
        rank,
        alpha,
        Some("Qwen/Qwen3.5-4B".into()),
        Some(provenance.clone()),
    );
    assert_eq!(v3.provenance, Some(provenance));
    let meta_path = dir.path().join("meta_v3.json");
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&v3).expect("serialize v3 manifest"),
    )
    .unwrap();

    let out_path = dir.path().join("merged_v3.safetensors");
    merge_qlora::run_merge_qlora(vec![base_path], ad_path, meta_path, out_path.clone())
        .expect("merge-qlora v3 meta");

    let mut delta = vec![0f32; vocab * d];
    for i in 0..vocab {
        for j in 0..d {
            let mut s = 0f32;
            for k in 0..rank {
                s += fb[i * rank + k] * fa[k * d + j];
            }
            delta[i * d + j] = s * scale;
        }
    }
    let bytes = std::fs::read(&out_path).unwrap();
    let st = SafeTensors::deserialize(&bytes).unwrap();
    let tv = st.tensor("wte.weight").unwrap();
    assert_eq!(tv.dtype(), Dtype::F32);
    let sl = tv.data();
    for i in 0..vocab * d {
        let o = i * 4;
        let got = f32::from_le_bytes([sl[o], sl[o + 1], sl[o + 2], sl[o + 3]]);
        let exp = w[i] + delta[i];
        assert!(
            (got - exp).abs() < 1e-5,
            "idx {i}: expected {exp} got {got}"
        );
    }
}

#[test]
fn eval_local_missing_model_errors() {
    let result = eval_local::run_eval_local(
        PathBuf::from("/nonexistent/model.bin"),
        PathBuf::from("mens/data/heldout_bench"),
        32,
        0.0,
        1,
        1337,
        None,
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not found") || msg.contains("Model"),
        "expected model not found: {msg}"
    );
}
