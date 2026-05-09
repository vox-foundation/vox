use vox_claim_extractor::pipeline::{ExtractionConfig, ExtractionPipeline};

struct EvalExample {
    sentence: &'static str,
    context: &'static str,
    expected_promotable: bool,
}

fn mini_scifact_split() -> Vec<EvalExample> {
    vec![
        EvalExample {
            sentence: "p95 latency rose by 15ms after the provider updated their model.",
            context: "We measured p95 latency before and after the provider update and found a 15ms increase.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "Tool call malformation rate increased from 0.8% to 2.3%.",
            context: "Tool call malformation rate was 0.8% in March and 2.3% in April.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "JSON mode violations increased by 40% over the measurement period.",
            context: "JSON schema violations rose 40% compared to baseline period.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "Model output token count rose by 120 tokens per call on average.",
            context: "Average output tokens per call increased from 380 to 500.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "Refusal rate dropped from 3.1% to 0.9% after the system prompt update.",
            context: "Refusal rate decreased from 3.1% to 0.9% following system prompt changes.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "Cost per million tokens increased by $0.50 for the provider in Q1.",
            context: "Provider pricing increased $0.50 per million tokens in Q1 2026.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "Cache hit rate improved by 8% after enabling prompt caching.",
            context: "Enabling prompt caching raised the cache hit rate by 8 percentage points.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "End-to-end latency fell by 200ms after switching to streaming mode.",
            context: "Switching to streaming reduced end-to-end latency by 200ms.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "Test pass rate fell from 92% to 78% after the model update.",
            context: "Tests that passed the suite dropped from 92% to 78% post-update.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "Throughput increased by 3.2 requests per second under peak load.",
            context: "Under peak load, throughput improved by 3.2 RPS.",
            expected_promotable: true,
        },
        EvalExample {
            sentence: "Future work could explore whether this generalizes to other providers.",
            context: "This is a future research direction.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "It may be possible that improvements exist in certain scenarios.",
            context: "Some scenarios may benefit.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "We believe this approach could potentially improve outcomes.",
            context: "The approach may help in some cases.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "This paper presents a novel framework for evaluation.",
            context: "A new framework is introduced.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "Motivated by these observations, we propose a new method.",
            context: "The method is motivated by prior observations.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "In some cases, the system might perform differently.",
            context: "Performance varies.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "Perhaps there are additional factors we have not considered.",
            context: "Additional factors exist.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "We hope that this work inspires further research.",
            context: "Further research is encouraged.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "Building on prior work, we aim to investigate this phenomenon.",
            context: "The work builds on prior research.",
            expected_promotable: false,
        },
        EvalExample {
            sentence: "It appears that the results are somewhat consistent with expectations.",
            context: "Results appear roughly consistent.",
            expected_promotable: false,
        },
    ]
}

#[tokio::test]
async fn scifact_mini_split_f1_above_065() {
    let pipeline = ExtractionPipeline::new(ExtractionConfig::default());
    let examples = mini_scifact_split();
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut fn_ = 0usize;
    let mut tn = 0usize;

    for ex in &examples {
        let result = pipeline.extract(ex.sentence, &[ex.context]).await.unwrap();
        let predicted_promotable = !result.promotable_claim_ids.is_empty();
        match (predicted_promotable, ex.expected_promotable) {
            (true, true) => tp += 1,
            (true, false) => fp += 1,
            (false, true) => fn_ += 1,
            (false, false) => tn += 1,
        }
    }

    let precision = if tp + fp > 0 {
        tp as f64 / (tp + fp) as f64
    } else {
        0.0
    };
    let recall = if tp + fn_ > 0 {
        tp as f64 / (tp + fn_) as f64
    } else {
        0.0
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    eprintln!("SciFact mini split: TP={tp} FP={fp} FN={fn_} TN={tn}");
    eprintln!("Precision={precision:.3} Recall={recall:.3} F1={f1:.3}");
    assert!(
        f1 >= 0.65,
        "F1={f1:.3} is below Phase 1 acceptance gate of 0.65"
    );
}
