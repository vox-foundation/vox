//! Multi-variant isolated worktree execution pool (`Muse` 1–4 variant fan-out, `T3-WORKTREE-VCS-03`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Minimum allowed parallel Muse worktree variants.
pub const MIN_MUSE_VARIANTS: usize = 1;
/// Maximum allowed parallel Muse worktree variants.
pub const MAX_MUSE_VARIANTS: usize = 4;

/// A single isolated worktree variant in a Muse multi-variant run (`T3-WORKTREE-VCS-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MuseVariantSlot {
    pub variant_index: usize,
    pub model_id: String,
    pub branch_name: String,
    pub worktree_path: PathBuf,
    pub adopted: bool,
}

/// Manages 1–4 isolated worktree variants for parallel multi-model code comparison (`T3-WORKTREE-VCS-03`).
#[derive(Debug, Clone, Default)]
pub struct MultiVariantWorktreePool {
    variants: Vec<MuseVariantSlot>,
}

impl MultiVariantWorktreePool {
    /// Plans `1..=4` isolated worktree slots under `<repo_root>/.vox/muse-worktrees/<run_id>-v<idx>`.
    pub fn plan_variants(
        repo_root: &Path,
        run_id: &str,
        model_ids: &[String],
    ) -> Result<Self, String> {
        if !(MIN_MUSE_VARIANTS..=MAX_MUSE_VARIANTS).contains(&model_ids.len()) {
            return Err(format!(
                "MultiVariantWorktreePool requires {}..={} model variants, got {}",
                MIN_MUSE_VARIANTS,
                MAX_MUSE_VARIANTS,
                model_ids.len()
            ));
        }
        let mut variants = Vec::with_capacity(model_ids.len());
        for (i, model_id) in model_ids.iter().enumerate() {
            let idx = i + 1;
            let slug = format!("{run_id}-v{idx}");
            let branch_name = format!("vox/muse/{slug}");
            let worktree_path = repo_root.join(".vox").join("muse-worktrees").join(&slug);
            variants.push(MuseVariantSlot {
                variant_index: idx,
                model_id: model_id.clone(),
                branch_name,
                worktree_path,
                adopted: false,
            });
        }
        Ok(Self { variants })
    }

    #[must_use]
    pub fn variants(&self) -> &[MuseVariantSlot] {
        &self.variants
    }

    /// Adopts the chosen variant (`1..=variants.len()`) as the winner and returns the losing
    /// variant paths that should be cleaned up and pruned (`T3-WORKTREE-VCS-03`).
    pub fn adopt_variant(
        &mut self,
        variant_index: usize,
    ) -> Result<(MuseVariantSlot, Vec<PathBuf>), String> {
        if !self
            .variants
            .iter()
            .any(|v| v.variant_index == variant_index)
        {
            return Err(format!("unknown variant_index {variant_index}"));
        }
        let mut winner = None;
        let mut losers = Vec::new();
        for slot in &mut self.variants {
            if slot.variant_index == variant_index {
                slot.adopted = true;
                winner = Some(slot.clone());
            } else {
                slot.adopted = false;
                losers.push(slot.worktree_path.clone());
            }
        }
        Ok((winner.expect("verified above"), losers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muse_pool_enforces_1_to_4_bounds_and_adopts_winner() {
        let root = Path::new("/repo");
        assert!(MultiVariantWorktreePool::plan_variants(root, "r1", &[]).is_err());
        assert!(
            MultiVariantWorktreePool::plan_variants(
                root,
                "r1",
                &[
                    "m1".into(),
                    "m2".into(),
                    "m3".into(),
                    "m4".into(),
                    "m5".into()
                ]
            )
            .is_err()
        );

        let mut pool = MultiVariantWorktreePool::plan_variants(
            root,
            "run42",
            &["claude".into(), "gpt".into(), "gemini".into()],
        )
        .unwrap();
        assert_eq!(pool.variants().len(), 3);

        let (winner, losers) = pool.adopt_variant(2).unwrap();
        assert_eq!(winner.model_id, "gpt");
        assert!(winner.adopted);
        assert_eq!(losers.len(), 2);
    }
}
