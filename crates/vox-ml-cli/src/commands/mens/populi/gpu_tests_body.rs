use crate::commands::mens::eval_local;
use crate::commands::mens::probe;
use crate::commands::mens::status;
use crate::commands::schola::merge_qlora;
use std::path::PathBuf;

#[test]
fn probe_runs_without_gpu() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(probe::run_probe(false, None, 512, false));
    assert!(result.is_ok());
}

#[test]
fn probe_verbose_runs_without_gpu() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // No --model: must stay the old VRAM-only profile path, which needs no
    // network and always succeeds — it must NOT attempt a download.
    let result = rt.block_on(probe::run_probe(true, None, 512, false));
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
        false,
    ));
    assert!(result.is_ok());
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
    let result = merge_qlora::run_merge_qlora(
        vec![base],
        adapter,
        meta,
        out,
        None,
        false,
        None,
        None,
        None,
    );
    assert!(result.is_err(), "expected rejection of Burn bin adapter");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("safetensors") || msg.contains("Candle"),
        "expected Candle safetensors hint: {msg}"
    );
    assert!(
        msg.contains("retired") || msg.contains("Burn"),
        "expected Burn-retired notice: {msg}"
    );
}

/// The `MlBackend` plugin for this host, taken from the workspace build (the dylib next to
/// this test binary's profile dir) and installed into a temp root. The user's plugin
/// install dir is never consulted, so a stale or ABI-mismatched install cannot be loaded.
/// `None` (after a SKIP line) when this host has no backend candidate, the plugin is not
/// built, or the build does not load against this host.
fn fresh_ml_backend() -> Option<(
    &'static str,
    vox_plugin_host::LoadedCodePlugin,
    tempfile::TempDir,
)> {
    let Ok(id) = vox_plugin_host::resolve_extension_point(
        "MlBackend",
        merge_qlora::ML_BACKEND_CANDIDATES,
        &vox_plugin_host::probe(),
    ) else {
        eprintln!("SKIP merge-qlora roundtrip: no MlBackend plugin candidate for this host");
        return None;
    };
    let lib = format!(
        "{}vox_plugin_{}{}",
        std::env::consts::DLL_PREFIX,
        id.replace('-', "_"),
        std::env::consts::DLL_SUFFIX
    );
    let exe = std::env::current_exe().expect("current_exe");
    let built = exe.parent()?.parent()?.join(&lib);
    if !built.is_file() {
        eprintln!(
            "SKIP merge-qlora roundtrip: {} not built; run `cargo build -p vox-plugin-{id}`",
            built.display()
        );
        return None;
    }
    let root = tempfile::tempdir().expect("plugins tempdir");
    let install = root.path().join(id);
    std::fs::create_dir_all(&install).expect("mkdir install");
    let manifest =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../vox-plugin-{id}/Plugin.toml"));
    std::fs::copy(&manifest, install.join("Plugin.toml")).expect("copy Plugin.toml");
    std::fs::copy(&built, install.join(&lib)).expect("copy plugin dylib");
    let registry = vox_plugin_host::discover(root.path()).expect("discover temp plugin root");
    match vox_plugin_host::load_code_plugin(&registry, id) {
        Ok(plugin) => Some((id, plugin, root)),
        Err(e) => {
            eprintln!(
                "SKIP merge-qlora roundtrip: {} does not load against this host ({e}); \
                 rebuild with `cargo build -p vox-plugin-{id}`",
                built.display()
            );
            None
        }
    }
}

#[test]
fn merge_qlora_cli_roundtrip_lm_head_subset() {
    use std::collections::HashMap;

    use safetensors::SafeTensors;
    use safetensors::tensor::{Dtype, TensorView};
    use serde_json::json;

    let Some((plugin_id, plugin, _plugin_root)) = fresh_ml_backend() else {
        return;
    };
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
    let base_path = dir.path().join("model.safetensors");
    std::fs::write(&base_path, safetensors::serialize(&base_map, None).unwrap()).unwrap();

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
        "lm_head.lora_a.weight".into(),
        TensorView::new(Dtype::F32, vec![rank, d], ab.as_slice()).unwrap(),
    );
    ad_map.insert(
        "lm_head.lora_b.weight".into(),
        TensorView::new(Dtype::F32, vec![vocab, rank], bb.as_slice()).unwrap(),
    );
    let ad_path = dir.path().join("adapter.safetensors");
    std::fs::write(&ad_path, safetensors::serialize(&ad_map, None).unwrap()).unwrap();

    let meta_path = dir.path().join("meta.json");
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&json!({
            "format": "vox_mens_adapter",
            "version": 3,
            "adapter_method": "qlora",
            "quant": { "base_quant": "nf4", "double_quant": true },
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
    merge_qlora::run_merge_qlora_using(
        Some((plugin_id, &plugin)),
        vec![base_path],
        ad_path,
        meta_path,
        out_path.clone(),
        None,
        false,
        None,
        None,
        None,
    )
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
    // The plugin unties tied embeddings: a merged "lm_head" logical layer is always
    // written back as "lm_head.weight", never the base_key_map source key ("wte.weight")
    // — see crates/vox-plugin-mens-candle-core/src/merge.rs merge_qlora_into_base_subset
    // and its own `merge_v2_applies_lm_head_delta` test.
    let tv = st.tensor("lm_head.weight").unwrap();
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

    let Some((plugin_id, plugin, _plugin_root)) = fresh_ml_backend() else {
        return;
    };
    use safetensors::SafeTensors;
    use safetensors::tensor::{Dtype, TensorView};

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
    let base_path = dir.path().join("model.safetensors");
    std::fs::write(&base_path, safetensors::serialize(&base_map, None).unwrap()).unwrap();

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
        "lm_head.lora_a.weight".into(),
        TensorView::new(Dtype::F32, vec![rank, d], ab.as_slice()).unwrap(),
    );
    ad_map.insert(
        "lm_head.lora_b.weight".into(),
        TensorView::new(Dtype::F32, vec![vocab, rank], bb.as_slice()).unwrap(),
    );
    let ad_path = dir.path().join("adapter.safetensors");
    std::fs::write(&ad_path, safetensors::serialize(&ad_map, None).unwrap()).unwrap();

    // Build a v3 manifest JSON directly (no typed enum imports from vox-populi needed).
    let v3_json = serde_json::json!({
        "format": "vox_mens_adapter",
        "version": 3,
        "adapter_method": "qlora",
        "quant": { "base_quant": "nf4", "double_quant": true },
        "base_key_map": { "lm_head": "wte.weight" },
        "layer_order": ["lm_head"],
        "vocab": vocab,
        "d_model": d,
        "rank": rank,
        "alpha": alpha,
        "base_model": "Qwen/Qwen3.5-4B",
        "provenance": {
            "base_family": "kimi-k2.5",
            "upstream_model_id": "moonshotai/Kimi-K2.5",
            "license_class": "modified-mit",
            "attribution_required": true
        }
    });
    let meta_path = dir.path().join("meta_v3.json");
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&v3_json).expect("serialize v3 manifest"),
    )
    .unwrap();

    let out_path = dir.path().join("merged_v3.safetensors");
    merge_qlora::run_merge_qlora_using(
        Some((plugin_id, &plugin)),
        vec![base_path],
        ad_path,
        meta_path,
        out_path.clone(),
        None,
        false,
        None,
        None,
        None,
    )
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
    // Same untie-on-merge behavior as the non-v3 roundtrip above: output key is
    // "lm_head.weight", not the base_key_map source key.
    let tv = st.tensor("lm_head.weight").unwrap();
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
        Some(PathBuf::from("/nonexistent/model.bin")),
        None,
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
