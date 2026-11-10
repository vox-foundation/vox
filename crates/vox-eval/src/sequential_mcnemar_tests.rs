use super::*;

#[test]
fn critical_losses_reproduces_research_boundary_table() {
    let cases: &[(f64, &[usize])] = &[
        // alpha 0.05 at look 128: gains 0..8 min losses
        (0.05, &[5, 7, 9, 10, 12, 13, 15, 16, 18]),
        // look_alpha 0.0005: gains 0..8 min losses
        (0.0005, &[11, 14, 17, 19, 21, 23, 25, 27, 29]),
        // look_alpha 0.002: gains 0..8 min losses
        (0.002, &[9, 12, 15, 17, 19, 21, 22, 24, 26]),
        // look_alpha 0.005: gains 0..8 min losses
        (0.005, &[8, 11, 13, 15, 17, 19, 20, 22, 24]),
        // look_alpha 0.04795: gains 0..8 min losses
        (0.04795, &[5, 7, 9, 10, 12, 14, 15, 16, 18]),
    ];

    for &(level, min_losses_by_gains) in cases {
        for (gains, &min_losses) in min_losses_by_gains.iter().enumerate() {
            let d = gains + min_losses;
            let critical = critical_losses(d, level);
            assert_eq!(
                critical, min_losses,
                "failed for level={level}, gains={gains}, d={d}: expected {min_losses}, got {critical}"
            );
        }
    }

    // Critical losses returns d + 1 when no loss count rejects
    assert_eq!(critical_losses(4, 0.05), 5);
    assert_eq!(critical_losses(0, 0.05), 1);
}

#[test]
fn from_fractions_derives_looks() {
    let rule = SequentialRule::from_fractions(
        &[0.25, 0.5, 0.75, 1.0],
        vec![0.0005, 0.002, 0.005, 0.04795],
        0.05,
        128,
    )
    .unwrap();
    assert_eq!(rule.looks(), &[32, 64, 96, 128]);
    assert_eq!(rule.look_alpha(), &[0.0005, 0.002, 0.005, 0.04795]);
    assert_eq!(rule.alpha(), 0.05);
}

#[test]
fn rejection_comparison_is_strictly_less() {
    let rule = SequentialRule::new(vec![32], vec![0.5], 0.5).unwrap();
    // 1 loss, 0 gains -> p = 0.5 exactly.
    // Under strict `<`, p < look_alpha is false when p == look_alpha.
    assert!(!rule.rejects_at(0, 1, 0));
    assert_eq!(critical_losses(1, 0.5), 2);
}

#[test]
fn structural_validation_errors() {
    // Empty looks
    assert!(SequentialRule::new(vec![], vec![], 0.05).is_err());
    // Mismatched lengths
    assert!(SequentialRule::new(vec![32, 64], vec![0.01], 0.05).is_err());
    // Looks not strictly increasing
    assert!(SequentialRule::new(vec![32, 32], vec![0.01, 0.02], 0.05).is_err());
    assert!(SequentialRule::new(vec![64, 32], vec![0.01, 0.02], 0.05).is_err());
    // First look is 0
    assert!(SequentialRule::new(vec![0, 32], vec![0.01, 0.02], 0.05).is_err());
    // Alpha out of range
    assert!(SequentialRule::new(vec![32], vec![0.01], 0.0).is_err());
    assert!(SequentialRule::new(vec![32], vec![0.01], 1.0).is_err());
    // Look alpha out of range
    assert!(SequentialRule::new(vec![32], vec![0.0], 0.05).is_err());
    assert!(SequentialRule::new(vec![32], vec![0.06], 0.05).is_err());

    // from_fractions specific validations:
    // Fraction outside (0, 1]
    assert!(SequentialRule::from_fractions(&[0.0, 1.0], vec![0.01, 0.02], 0.05, 128).is_err());
    assert!(SequentialRule::from_fractions(&[0.5, 1.1], vec![0.01, 0.02], 0.05, 128).is_err());
    // Last fraction not 1.0
    assert!(SequentialRule::from_fractions(&[0.25, 0.5], vec![0.01, 0.02], 0.05, 128).is_err());
    // Fractions not strictly increasing
    assert!(
        SequentialRule::from_fractions(&[0.5, 0.25, 1.0], vec![0.01, 0.02, 0.03], 0.05, 128)
            .is_err()
    );
    // Empty fractions
    assert!(SequentialRule::from_fractions(&[], vec![], 0.05, 128).is_err());
}

#[test]
fn judge_stops_at_look_1() {
    let rule = SequentialRule::new(
        vec![32, 64, 96, 128],
        vec![0.0005, 0.002, 0.005, 0.04795],
        0.05,
    )
    .unwrap();

    // 11 Loss, 21 Tie out of 32 items:
    // critical_losses(11, 0.0005) is 11 (losses=11, gains=0). p = (0.5)^11 = 0.000488 < 0.0005. Rejects!
    let mut outcomes = vec![PairOutcome::Loss; 11];
    outcomes.resize(32, PairOutcome::Tie);

    let verdict = rule.judge(&outcomes);
    assert_eq!(verdict.stopped_at_look, Some(1));
    assert_eq!(verdict.items_used, 32);
    assert!(verdict.complete);
    assert_eq!(verdict.looks.len(), 1);
    assert!(verdict.looks[0].rejected);

    // 10 Loss, 22 Tie: p = 11 / 2^10 = 0.0107 >= 0.0005. Does not reject.
    let mut outcomes_10 = vec![PairOutcome::Loss; 10];
    outcomes_10.resize(32, PairOutcome::Tie);

    let verdict_10 = rule.judge(&outcomes_10);
    assert_eq!(verdict_10.stopped_at_look, None);
    assert_eq!(verdict_10.items_used, 32);
    assert!(!verdict_10.complete);
    assert_eq!(verdict_10.looks.len(), 1);
    assert!(!verdict_10.looks[0].rejected);
}

#[test]
fn judge_stops_at_look_4() {
    let rule = SequentialRule::new(
        vec![32, 64, 96, 128],
        vec![0.0005, 0.002, 0.005, 0.04795],
        0.05,
    )
    .unwrap();

    // 128 outcomes with 5 Loss and 0 Gain at the end (items 123..128) and Tie elsewhere.
    // At looks 1 (32), 2 (64), 3 (96): losses=0, gains=0 -> not rejected.
    // At look 4 (128): losses=5, gains=0 -> p = 0.03125 < 0.04795 -> rejected!
    let mut outcomes = vec![PairOutcome::Tie; 123];
    outcomes.extend(vec![PairOutcome::Loss; 5]);
    assert_eq!(outcomes.len(), 128);

    let verdict = rule.judge(&outcomes);
    assert_eq!(verdict.stopped_at_look, Some(4));
    assert_eq!(verdict.items_used, 128);
    assert!(verdict.complete);
    assert_eq!(verdict.looks.len(), 4);
    assert!(!verdict.looks[0].rejected);
    assert!(!verdict.looks[1].rejected);
    assert!(!verdict.looks[2].rejected);
    assert!(verdict.looks[3].rejected);
}

#[test]
fn judge_evaluates_only_covered_looks_and_ignores_past_rejection() {
    let rule = SequentialRule::new(
        vec![32, 64, 96, 128],
        vec![0.0005, 0.002, 0.005, 0.04795],
        0.05,
    )
    .unwrap();

    // 50 items: only look 1 (32 items) is covered
    let outcomes_50 = vec![PairOutcome::Tie; 50];
    let verdict_50 = rule.judge(&outcomes_50);
    assert_eq!(verdict_50.looks.len(), 1);
    assert_eq!(verdict_50.stopped_at_look, None);
    assert!(!verdict_50.complete);

    // 128 items where look 1 rejects: items 33..128 never alter the verdict
    let mut outcomes_128 = vec![PairOutcome::Loss; 11];
    outcomes_128.resize(32, PairOutcome::Tie);
    outcomes_128.resize(128, PairOutcome::Gain);
    let verdict_128 = rule.judge(&outcomes_128);
    assert_eq!(verdict_128.stopped_at_look, Some(1));
    assert_eq!(verdict_128.items_used, 32);
    assert_eq!(verdict_128.looks.len(), 1);
    assert!(verdict_128.complete);
}

#[test]
fn preregistered_schedule_worst_case_size() {
    let rule = SequentialRule::new(
        vec![32, 64, 96, 128],
        vec![0.0005, 0.002, 0.005, 0.04795],
        0.05,
    )
    .unwrap();

    let start = std::time::Instant::now();
    let size = rule.worst_case_size();
    let elapsed = start.elapsed();
    println!("preregistered schedule worst_case_size: {size} in {elapsed:?}");

    assert!(
        (size - 0.04884).abs() < 1e-5,
        "expected size within 1e-5 of 0.04884, got {size}"
    );
    assert!(rule.check_size().is_ok());
}

#[test]
fn single_look_128_worst_case_size() {
    let rule = SequentialRule::new(vec![128], vec![0.05], 0.05).unwrap();
    let size = rule.worst_case_size();
    assert!(
        (size - 0.0494).abs() < 1e-4,
        "expected size within 1e-4 of 0.0494, got {size}"
    );
    assert!(rule.check_size().is_ok());
}

#[test]
fn two_look_and_pocock_worst_case_size() {
    let rule_2 = SequentialRule::new(vec![64, 128], vec![0.005, 0.04795], 0.05).unwrap();
    let size_2 = rule_2.worst_case_size();
    assert!(
        (size_2 - 0.04941).abs() < 1e-5,
        "expected size within 1e-5 of 0.04941, got {size_2}"
    );

    let pocock = SequentialRule::new(
        vec![32, 64, 96, 128],
        vec![0.01734, 0.01734, 0.01734, 0.01734],
        0.05,
    )
    .unwrap();
    let size_pocock = pocock.worst_case_size();
    assert!(
        (size_pocock - 0.04937).abs() < 1e-5,
        "expected size within 1e-5 of 0.04937, got {size_pocock}"
    );
}

#[test]
fn check_size_rejects_oversized_schedule() {
    let rule = SequentialRule::new(
        vec![32, 64, 96, 128],
        vec![0.05, 0.002, 0.005, 0.04795],
        0.05,
    )
    .unwrap();
    let err = rule.check_size().unwrap_err();
    assert!(
        err.contains("exceeds alpha") && err.contains("0.05"),
        "got: {err}"
    );
}

#[test]
fn brute_force_cross_check_equals_worst_case_size() {
    let rule = SequentialRule::new(vec![4, 8, 12], vec![0.02, 0.03, 0.04], 0.05).unwrap();

    let dp_size = rule.worst_case_size();

    // Brute force: enumerate all 4096 discordance masks over 12 items
    let mut brute_max = 0.0f64;
    for mask in 0..4096usize {
        let mut disc_indices = Vec::new();
        for i in 0..12 {
            if (mask & (1 << i)) != 0 {
                disc_indices.push(i);
            }
        }
        let d_total = disc_indices.len();
        if d_total == 0 {
            continue;
        }

        let num_seqs = 1usize << d_total;
        let mut rej_count = 0usize;
        for seq in 0..num_seqs {
            let mut l1 = 0;
            let mut g1 = 0;
            let mut l2 = 0;
            let mut g2 = 0;
            let mut l3 = 0;
            let mut g3 = 0;

            for (bit_idx, &item_idx) in disc_indices.iter().enumerate() {
                let is_loss = (seq & (1 << bit_idx)) != 0;
                if item_idx < 4 {
                    if is_loss {
                        l1 += 1;
                    } else {
                        g1 += 1;
                    }
                }
                if item_idx < 8 {
                    if is_loss {
                        l2 += 1;
                    } else {
                        g2 += 1;
                    }
                }
                if item_idx < 12 {
                    if is_loss {
                        l3 += 1;
                    } else {
                        g3 += 1;
                    }
                }
            }

            if mcnemar_one_sided_p(l1, g1) < rule.look_alpha()[0]
                || mcnemar_one_sided_p(l2, g2) < rule.look_alpha()[1]
                || mcnemar_one_sided_p(l3, g3) < rule.look_alpha()[2]
            {
                rej_count += 1;
            }
        }

        let p_rej = rej_count as f64 / num_seqs as f64;
        if p_rej > brute_max {
            brute_max = p_rej;
        }
    }

    assert!(
        (dp_size - brute_max).abs() < 1e-12,
        "DP size ({dp_size:.15}) does not equal brute force ({brute_max:.15}) to 1e-12; diff = {:.2e}",
        (dp_size - brute_max).abs()
    );
}

#[test]
fn verify_sequential_block_rejudges_from_counts() {
    let rule = SequentialRule::new(
        vec![32, 64, 96, 128],
        vec![0.0005, 0.002, 0.005, 0.04795],
        0.05,
    )
    .unwrap();

    let outcomes = vec![PairOutcome::Tie; 128];
    let verdicts: Vec<SequentialSliceVerdict> = crate::reasoning_bench::SLICE_NAMES
        .iter()
        .map(|name| rule.judge_slice(name, &outcomes, &outcomes))
        .collect();

    let block = sequential_slices_json(&verdicts, None, &rule);

    // 1. Verifies Ok on a pass and rebuilds verdicts from last look counts
    let res = verify_sequential_block(&block, 0.05);
    assert!(res.is_ok(), "expected Ok, got {res:?}");
    let reconstructed = res.unwrap();
    assert_eq!(reconstructed.len(), 3);
    for v in &reconstructed {
        assert_eq!(v.n, 128);
        assert_eq!(v.accuracy.losses, 0);
        assert_eq!(v.accuracy.gains, 0);
        assert!(!v.accuracy.regressed);
        assert_eq!(v.think_initiation.losses, 0);
        assert_eq!(v.think_initiation.gains, 0);
        assert!(!v.think_initiation.regressed);
    }

    // 2. Editing a stored `rejected` flag or `p` changes nothing (they are recomputed)
    let mut tampered = block.clone();
    tampered["benches"]["gsm8k_think"]["accuracy"]["looks"][0]["rejected"] =
        serde_json::json!(true);
    tampered["benches"]["gsm8k_think"]["accuracy"]["looks"][0]["p"] = serde_json::json!(0.000001);
    assert!(verify_sequential_block(&tampered, 0.05).is_ok());

    // 3. Editing a stored `losses` so a look rejects makes verify Err naming the slice and look
    let mut regressed = block.clone();
    regressed["benches"]["gsm8k_think"]["accuracy"]["looks"][0]["losses"] = serde_json::json!(11);
    regressed["benches"]["gsm8k_think"]["stopped_at_look"] = serde_json::json!(1);
    regressed["benches"]["gsm8k_think"]["items_used"] = serde_json::json!(32);
    let err = verify_sequential_block(&regressed, 0.05).unwrap_err();
    assert!(err.contains("gsm8k_think"), "{err}");
    assert!(err.contains("look 1"), "{err}");

    // 4. Stored schedule whose worst-case size exceeds alpha is Err
    let mut bad_schedule = block.clone();
    bad_schedule["sequential"]["look_alpha"][3] = serde_json::json!(0.05);
    let err_sched = verify_sequential_block(&bad_schedule, 0.05).unwrap_err();
    assert!(err_sched.contains("exceeds alpha"), "{err_sched}");
}

#[test]
fn an_unknown_rule_is_refused() {
    let report = serde_json::json!({
        "reasoning_slices": {
            "rule": "not_a_valid_rule",
            "alpha": 0.05,
        }
    });
    let err = crate::reasoning_slice::verify_reasoning_slices(&report, 0.05).unwrap_err();
    assert!(
        err.contains("not a known rule") || err.contains("not_a_valid_rule"),
        "{err}"
    );
}
