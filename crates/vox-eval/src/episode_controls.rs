//! Scripted oracle and nine named known-bad controls for agent episode validation (`D-10`).

use crate::episode::{ChatMessage, Episode, ToolDef, episode_nonce, resolve_reference_turn};
use serde_json::{Map, Value};

pub type BoxedPolicy<'a> =
    Box<dyn FnMut(&[ChatMessage], &[ToolDef]) -> Result<String, String> + 'a>;

fn seeded_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn find_ref_tool_call(ep: &Episode, seed: u64, idx: usize) -> Option<crate::episode::ToolCall> {
    let raw = ep.reference.get(idx)?;
    let resolved = resolve_reference_turn(raw, seed, &ep.id);
    crate::episode::parse_tool_call(&resolved)
}

/// Scripted oracle replaying `ep.reference` with per-seed `${nonce}` substitution.
#[must_use]
pub fn oracle(ep: &Episode, seed: u64) -> BoxedPolicy<'_> {
    let mut turn = 0usize;
    let id = ep.id.clone();
    Box::new(move |_msgs, _tools| {
        let reply = ep
            .reference
            .get(turn)
            .map(|r| resolve_reference_turn(r, seed, &id))
            .unwrap_or_else(|| "Done.".to_string());
        turn += 1;
        Ok(reply)
    })
}

/// Control 1: replies with an empty terminal turn and never calls a tool.
#[must_use]
pub fn never_call(_ep: &Episode, _seed: u64) -> BoxedPolicy<'static> {
    Box::new(|_msgs, _tools| Ok(String::new()))
}

/// Control 2: refuses every request without calling any tool.
#[must_use]
pub fn refuse_everything(_ep: &Episode, _seed: u64) -> BoxedPolicy<'static> {
    Box::new(|_msgs, _tools| Ok("I can't help with that.".to_string()))
}

/// Control 3: shifts every offered reference tool name to the next offered tool while keeping arguments.
#[must_use]
pub fn shuffled_tool(ep: &Episode, seed: u64) -> BoxedPolicy<'_> {
    let mut turn = 0usize;
    let id = ep.id.clone();
    Box::new(move |_msgs, tools| {
        let Some(raw) = ep.reference.get(turn) else {
            turn += 1;
            return Ok("Done.".to_string());
        };
        let resolved = resolve_reference_turn(raw, seed, &id);
        turn += 1;
        if let Some(mut tc) = crate::episode::parse_tool_call(&resolved)
            && tools.len() >= 2
        {
            if let Some(pos) = tools.iter().position(|t| t.name == tc.name) {
                tc.name = tools[(pos + 1) % tools.len()].name.clone();
            } else {
                tc.name = tools[0].name.clone();
            }
            return Ok(serde_json::json!({
                "name": tc.name,
                "arguments": tc.arguments,
            })
            .to_string());
        }
        Ok(resolved)
    })
}

/// Control 4: always calls the first offered tool with empty `{}` arguments.
#[must_use]
pub fn first_offered_tool(_ep: &Episode, _seed: u64) -> BoxedPolicy<'static> {
    let mut called = false;
    Box::new(move |_msgs, tools| {
        if called {
            return Ok("Completed.".to_string());
        }
        called = true;
        let name = tools
            .first()
            .map(|t| t.name.as_str())
            .unwrap_or("vox_git_status");
        Ok(serde_json::json!({ "name": name, "arguments": {} }).to_string())
    })
}

/// Control 5: picks a pseudorandom offered tool and synthesizes wrong-typed/incomplete arguments from `seed`.
#[must_use]
pub fn random_seeded(ep: &Episode, seed: u64) -> BoxedPolicy<'static> {
    let mut state = seed ^ (xxhash_rust::xxh3::xxh3_64(ep.id.as_bytes()) | 1);
    let mut turns = 0usize;
    Box::new(move |_msgs, tools| {
        turns += 1;
        if turns > 1 || tools.is_empty() {
            return Ok("Finished.".to_string());
        }
        let pick = (seeded_next(&mut state) as usize) % tools.len();
        let tool = &tools[pick];
        let mut args = Map::new();
        if let Some(props) = tool
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            && let Some((k, _)) = props.iter().next()
        {
            // Intentionally emit a bogus integer for likely string keys or wrong value
            let v = seeded_next(&mut state) % 999;
            args.insert(k.clone(), Value::Number(v.into()));
        }
        Ok(serde_json::json!({ "name": tool.name, "arguments": Value::Object(args) }).to_string())
    })
}

/// Control 6: replays the first reference call's tool and arguments on every turn, ignoring tool outputs.
#[must_use]
pub fn ignore_results(ep: &Episode, seed: u64) -> BoxedPolicy<'_> {
    let mut turn = 0usize;
    let id = ep.id.clone();
    Box::new(move |_msgs, _tools| {
        let idx = turn;
        turn += 1;
        let Some(raw) = ep.reference.get(idx) else {
            return Ok("Done.".to_string());
        };
        let resolved = resolve_reference_turn(raw, seed, &id);
        if idx >= 1
            && let Some(first_tc) = find_ref_tool_call(ep, seed, 0)
            && crate::episode::parse_tool_call(&resolved).is_some()
        {
            return Ok(serde_json::json!({
                "name": first_tc.name,
                "arguments": first_tc.arguments,
            })
            .to_string());
        }
        Ok(resolved)
    })
}

/// Control 7: repeats the failing first call instead of repairing arguments or falling back.
#[must_use]
pub fn no_recovery(ep: &Episode, seed: u64) -> BoxedPolicy<'_> {
    let mut turn = 0usize;
    let first = find_ref_tool_call(ep, seed, 0);
    let id = ep.id.clone();
    Box::new(move |_msgs, _tools| {
        let idx = turn;
        turn += 1;
        if let Some(tc) = &first
            && idx < 2
        {
            return Ok(serde_json::json!({
                "name": tc.name,
                "arguments": tc.arguments,
            })
            .to_string());
        }
        Ok(ep
            .reference
            .get(idx)
            .map(|r| resolve_reference_turn(r, seed, &id))
            .unwrap_or_else(|| "Giving up after error.".to_string()))
    })
}

/// Control 8: keeps calling the reference's first tool on every turn until `max_turns` exhausts.
#[must_use]
pub fn call_forever(ep: &Episode, seed: u64) -> BoxedPolicy<'static> {
    let first = find_ref_tool_call(ep, seed, 0).unwrap_or_else(|| crate::episode::ToolCall {
        name: ep
            .offered_tools
            .first()
            .cloned()
            .unwrap_or_else(|| "vox_git_status".to_string()),
        arguments: serde_json::json!({}),
    });
    let payload = serde_json::json!({
        "name": first.name,
        "arguments": first.arguments,
    })
    .to_string();
    Box::new(move |_msgs, _tools| Ok(payload.clone()))
}

/// Control 9: completes the oracle sequence, then makes one extra redundant tool call before finishing.
#[must_use]
pub fn extra_call_after_goal(ep: &Episode, seed: u64) -> BoxedPolicy<'_> {
    let mut turn = 0usize;
    let id = ep.id.clone();
    let call_turns: usize = ep
        .reference
        .iter()
        .take_while(|r| {
            crate::episode::parse_tool_call(&resolve_reference_turn(r, seed, &id)).is_some()
        })
        .count();
    let last_call = if call_turns > 0 {
        ep.reference
            .get(call_turns - 1)
            .map(|r| resolve_reference_turn(r, seed, &id))
    } else {
        None
    };

    Box::new(move |_msgs, tools| {
        let idx = turn;
        turn += 1;
        if idx < call_turns {
            return Ok(resolve_reference_turn(&ep.reference[idx], seed, &id));
        }
        if idx == call_turns {
            if let Some(call) = &last_call {
                return Ok(call.clone());
            }
            let name = tools
                .first()
                .map(|t| t.name.as_str())
                .unwrap_or("vox_git_status");
            return Ok(serde_json::json!({ "name": name, "arguments": {} }).to_string());
        }
        Ok("Done with extra check.".to_string())
    })
}

/// Diagnostic helper verifying nonce computation is deterministic.
#[must_use]
pub fn seeded_nonce_preview(seed: u64, episode_id: &str) -> String {
    episode_nonce(seed, episode_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::{
        Behavior, load_catalog, load_episodes, run_episode, score_episode, toy_validate_schema,
    };
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn repo_data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("mens/data/agent_episodes")
    }

    #[test]
    fn control_matrix() {
        let dir = repo_data_dir();
        let cat_path = dir.join("tool-catalog.v1.json");
        let ep_path = dir.join("episodes.v1.jsonl");
        let cat_raw = std::fs::read_to_string(&cat_path).expect("tool-catalog.v1.json");
        let ep_raw = std::fs::read_to_string(&ep_path).expect("episodes.v1.jsonl");
        let cat_json: Value = serde_json::from_str(&cat_raw).expect("valid catalog json");
        let catalog = load_catalog(&cat_json).expect("load_catalog");
        let episodes = load_episodes(&ep_raw, &catalog).expect("load_episodes");

        assert_eq!(episodes.len(), 72, "expected 72 agent episodes");
        for b in Behavior::ALL {
            let count = episodes.iter().filter(|e| e.behavior == b).count();
            assert_eq!(count, 12, "expected 12 episodes for {:?}", b);
        }

        let seed = 42u64;
        type ControlFactory = (&'static str, fn(&Episode, u64) -> BoxedPolicy<'_>);
        let controls: Vec<ControlFactory> = vec![
            ("oracle", oracle),
            ("never_call", never_call),
            ("refuse_everything", refuse_everything),
            ("shuffled_tool", shuffled_tool),
            ("first_offered_tool", first_offered_tool),
            ("random_seeded", random_seeded),
            ("ignore_results", ignore_results),
            ("no_recovery", no_recovery),
            ("call_forever", call_forever),
            ("extra_call_after_goal", extra_call_after_goal),
        ];

        let mut matrix: BTreeMap<&'static str, (f64, BTreeMap<Behavior, f64>)> = BTreeMap::new();
        for (name, factory) in &controls {
            let mut total_pass = 0usize;
            let mut by_beh_counts: BTreeMap<Behavior, (usize, usize)> = BTreeMap::new();
            for ep in &episodes {
                let mut pol = factory(ep, seed);
                let out = run_episode(ep, &catalog, &mut *pol, &toy_validate_schema, seed);
                let sc = score_episode(ep, &out);
                let entry = by_beh_counts.entry(ep.behavior).or_insert((0, 0));
                entry.1 += 1;
                if sc.passed {
                    total_pass += 1;
                    entry.0 += 1;
                } else if *name == "oracle" {
                    panic!("oracle failed episode {}: {}", ep.id, sc.reason);
                }
            }
            let overall = total_pass as f64 / episodes.len() as f64;
            let beh_rates: BTreeMap<Behavior, f64> = by_beh_counts
                .into_iter()
                .map(|(b, (p, n))| (b, p as f64 / n as f64))
                .collect();
            matrix.insert(name, (overall, beh_rates));
        }

        let (oracle_overall, oracle_groups) = &matrix["oracle"];
        assert!(
            (*oracle_overall - 1.0).abs() < f64::EPSILON,
            "oracle overall={oracle_overall}"
        );
        for b in Behavior::ALL {
            assert!(
                (oracle_groups[&b] - 1.0).abs() < f64::EPSILON,
                "oracle {b:?}={}",
                oracle_groups[&b]
            );
        }

        let targeted: &[(&str, &[Behavior])] = &[
            ("never_call", &[Behavior::RightTool]),
            ("refuse_everything", &[Behavior::SchemaValidArgs]),
            ("shuffled_tool", &[Behavior::RightTool]),
            (
                "first_offered_tool",
                &[Behavior::RightTool, Behavior::RefusesOutOfScope],
            ),
            (
                "random_seeded",
                &[Behavior::SchemaValidArgs, Behavior::RefusesOutOfScope],
            ),
            ("ignore_results", &[Behavior::ConsumesResults]),
            ("no_recovery", &[Behavior::RecoversFromError]),
            ("call_forever", &[Behavior::StopsWhenDone]),
            ("extra_call_after_goal", &[Behavior::StopsWhenDone]),
        ];

        for (ctrl, target_behaviors) in targeted {
            let (overall, groups) = &matrix[ctrl];
            assert!(
                *overall < 0.80,
                "control {ctrl} scored {overall:.3} >= 0.80 overall"
            );
            for b in *target_behaviors {
                assert!(
                    groups[b] < 0.70,
                    "control {ctrl} scored {:.3} >= 0.70 on target {:?}",
                    groups[b],
                    b
                );
            }
        }
    }
}
