//! Projection-stack naming for Candle QLoRA export / merge (`mid0` … `lm_head`).

/// Logits tensor rank-3 shape after `QLoraTrainer::training_step_lm` chains all stacked layers:
/// `[batch, seq, d_model]` through square middle blocks, then LM head → `[batch, seq, vocab]`.
#[must_use]
pub fn stacked_lm_logits_shape(batch: usize, seq: usize, vocab: usize) -> [usize; 3] {
    [batch, seq, vocab]
}

/// Logical adapter names: one per middle projection, then LM head.
#[must_use]
pub fn adapter_names_for_stack(n_middle: usize) -> Vec<String> {
    let mut v: Vec<String> = (0..n_middle).map(|i| format!("mid{i}")).collect();
    v.push("lm_head".into());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_stack() {
        assert_eq!(adapter_names_for_stack(2), vec!["mid0", "mid1", "lm_head"]);
    }

    #[test]
    fn logits_rank3_contract() {
        assert_eq!(stacked_lm_logits_shape(1, 1, 32000), [1, 1, 32000]);
    }
}
