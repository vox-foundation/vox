//! `vox mens train --plan-only` (D-02): the run as resolved right before the
//! download, rendered as `key: value` text or one JSON object.

use std::path::Path;

use vox_corpus::training::preflight::{HeldoutCheck, ResolvedTrainInput};
use vox_populi::mens::tensor::preset_schema::TrainPresetProfile;
use vox_populi::mens::tensor::size_select::{FreeMemory, SizePick};

#[derive(Debug, serde::Serialize)]
pub(super) struct TrainPlan {
    schema: &'static str,
    model: Option<String>,
    size: Option<String>,
    explicit: Option<bool>,
    pick_reason: Option<String>,
    free_mb: Option<u64>,
    free_source: Option<&'static str>,
    device: String,
    preset: Option<String>,
    seq_len: usize,
    epochs: usize,
    batch_size: usize,
    grad_accum: usize,
    base_cached: Option<bool>,
    corpus_path: String,
    corpus_rows: usize,
    license_class: Option<String>,
    floor_notes: Vec<String>,
    contamination: HeldoutCheck,
}

impl TrainPlan {
    /// Collects the plan from what `run_gpu_training` resolved. `base_cached`
    /// is a local cache scan (no network); the floor notes are config reads.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        model: Option<&str>,
        pick: Option<&SizePick>,
        reading: Option<FreeMemory>,
        device: &str,
        preset: Option<&str>,
        profile: &TrainPresetProfile,
        corpus: &ResolvedTrainInput,
        license_class: Option<String>,
        workspace_root: Option<&Path>,
        contamination: HeldoutCheck,
    ) -> anyhow::Result<Self> {
        use vox_populi::mens::tensor::spoke_base_resolver::floor_notes;
        Ok(Self {
            schema: "vox_mens_train_plan_v1",
            model: model.map(str::to_string),
            size: pick.map(|p| p.rung.size.clone()),
            explicit: pick.map(|p| p.explicit),
            pick_reason: pick.map(|p| p.note.clone()),
            free_mb: reading.map(|r| r.mb),
            free_source: reading.map(|r| r.source),
            device: device.to_string(),
            preset: preset.map(str::to_string),
            seq_len: profile.seq_len,
            epochs: profile.epochs,
            batch_size: profile.batch_size,
            grad_accum: profile.grad_accum,
            base_cached: model.and_then(|m| vox_populi::mens::hub::is_model_cached(m).ok()),
            corpus_path: corpus.path.display().to_string(),
            corpus_rows: corpus.sample_count.unwrap_or(0),
            license_class,
            floor_notes: match model {
                Some(m) => floor_notes(workspace_root, m)?,
                None => Vec::new(),
            },
            contamination,
        })
    }

    pub(super) fn render(&self, json: bool) -> String {
        if json {
            return serde_json::to_string_pretty(self).expect("TrainPlan is plain data");
        }
        let or_none = |v: &Option<String>| v.clone().unwrap_or_else(|| "none".into());
        let free = match (self.free_mb, self.free_source) {
            (Some(mb), Some(src)) => format!("{mb} MB ({src})"),
            _ => "no reading".into(),
        };
        let mut out = vec![
            format!("plan-only: {} (nothing downloaded or trained)", self.schema),
            format!("model: {}", or_none(&self.model)),
            format!(
                "size: {} (explicit: {:?})",
                or_none(&self.size),
                self.explicit
            ),
            format!("pick reason: {}", or_none(&self.pick_reason)),
            format!("free memory: {free}"),
            format!("device: {}", self.device),
            format!("preset: {}", or_none(&self.preset)),
            format!("seq_len: {}", self.seq_len),
            format!("epochs: {}", self.epochs),
            format!(
                "batch_size x grad_accum: {} x {}",
                self.batch_size, self.grad_accum
            ),
            format!("base cached: {:?}", self.base_cached),
            format!("corpus: {} ({} rows)", self.corpus_path, self.corpus_rows),
            format!("license class: {}", or_none(&self.license_class)),
        ];
        out.push(format!("held-out check: {}", self.contamination));
        out.extend(self.floor_notes.iter().map(|n| format!("floor note: {n}")));
        out.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::{HeldoutCheck, TrainPlan};

    fn plan(free: Option<(u64, &'static str)>) -> TrainPlan {
        TrainPlan {
            schema: "vox_mens_train_plan_v1",
            model: Some("Qwen/Qwen3-1.7B@abc".into()),
            size: Some("1.7B".into()),
            explicit: Some(true),
            pick_reason: Some("Qwen3-1.7B (explicit; fit not checked)".into()),
            free_mb: free.map(|f| f.0),
            free_source: free.map(|f| f.1),
            device: "metal".into(),
            preset: Some("qwen3_1_7b".into()),
            seq_len: 3072,
            epochs: 1,
            batch_size: 1,
            grad_accum: 8,
            base_cached: Some(false),
            corpus_path: "/data/train.jsonl".into(),
            corpus_rows: 100,
            license_class: Some("apache-2.0".into()),
            floor_notes: vec!["floor estimated".into()],
            contamination: HeldoutCheck {
                status: "clean",
                slices: 3,
                heldout_questions: 96,
                rows_checked: 100,
            },
        }
    }

    #[test]
    fn json_plan_carries_every_d02_field() {
        let v: serde_json::Value =
            serde_json::from_str(&plan(Some((9000, "vm_stat"))).render(true)).expect("json");
        assert_eq!(v["schema"], "vox_mens_train_plan_v1");
        for key in [
            "model",
            "size",
            "explicit",
            "pick_reason",
            "free_mb",
            "free_source",
            "device",
            "preset",
            "seq_len",
            "epochs",
            "batch_size",
            "grad_accum",
            "base_cached",
            "corpus_path",
            "corpus_rows",
            "license_class",
            "floor_notes",
            "contamination",
        ] {
            assert!(v.get(key).is_some(), "missing {key}: {v}");
        }
        assert_eq!(v["free_mb"], 9000);
        assert_eq!(v["corpus_rows"], 100);
        assert_eq!(v["contamination"]["status"], "clean");
        assert_eq!(v["contamination"]["heldout_questions"], 96);
    }

    #[test]
    fn missing_reading_is_null_not_zero() {
        let v: serde_json::Value = serde_json::from_str(&plan(None).render(true)).expect("json");
        assert!(v["free_mb"].is_null(), "{v}");
        assert!(v["free_source"].is_null(), "{v}");
        assert!(plan(None).render(false).contains("free memory: no reading"));
    }

    #[test]
    fn text_plan_names_pick_memory_and_corpus() {
        let text = plan(Some((9000, "vm_stat"))).render(false);
        assert!(text.starts_with("plan-only:"), "{text}");
        assert!(
            text.contains("Qwen3-1.7B (explicit; fit not checked)"),
            "{text}"
        );
        assert!(text.contains("free memory: 9000 MB (vm_stat)"), "{text}");
        assert!(text.contains("/data/train.jsonl"), "{text}");
        assert!(text.contains("100 rows"), "{text}");
    }
}
