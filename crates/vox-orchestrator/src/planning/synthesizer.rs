use crate::planning::policy::default_policy_for_goal;
use crate::planning::{PlanNode, PlanStatus};

fn split_goal_clauses(goal: &str) -> Vec<String> {
    let mut pieces: Vec<String> = Vec::new();
    let normalized = goal.replace('\r', "");
    for block in normalized.split('\n') {
        let line = block.trim();
        if line.is_empty() {
            continue;
        }
        // Numbered list lines: "1. Do thing"
        let rest = if let Some((head, tail)) = line.split_once('.') {
            let is_ordered =
                head.trim().chars().all(|c| c.is_ascii_digit()) && !head.trim().is_empty();
            if is_ordered { tail.trim() } else { line }
        } else {
            line
        };
        for segment in rest.split(" and ") {
            let s = segment.trim().trim_end_matches('.').trim();
            if s.is_empty() {
                continue;
            }
            for sub in s.split(';') {
                let t = sub.trim().trim_end_matches('.').trim();
                if t.is_empty() {
                    continue;
                }
                for part in t.split(" then ") {
                    let p = part.trim().trim_end_matches('.').trim();
                    if !p.is_empty() {
                        pieces.push(p.to_string());
                    }
                }
            }
        }
    }
    pieces
}

pub fn synthesize_plan_nodes(goal: &str) -> Vec<PlanNode> {
    let mut parts = split_goal_clauses(goal);
    if parts.is_empty() {
        let trimmed = goal.trim();
        if trimmed.is_empty() {
            return vec![];
        }
        parts.push(trimmed.to_string());
    }

    let mut nodes = Vec::with_capacity(parts.len());
    for (idx, part) in parts.iter().enumerate() {
        let node_id = format!("n{}", idx + 1);
        let depends_on = if idx == 0 {
            vec![]
        } else {
            vec![format!("n{}", idx)]
        };
        nodes.push(PlanNode {
            node_id,
            description: part.clone(),
            depends_on,
            status: PlanStatus::Pending,
            execution_policy: default_policy_for_goal(part),
            workflow_invocation: None,
        });
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_semicolon_and_then() {
        let g = "run fmt; run tests then deploy";
        let n = synthesize_plan_nodes(g);
        assert!(n.len() >= 3, "expected multiple clauses, got {}", n.len());
        assert_eq!(n[0].depends_on.len(), 0);
        assert_eq!(n[1].depends_on, vec!["n1".to_string()]);
    }

    #[test]
    fn numbered_lines_become_chain() {
        let g = "1. First\n2. Second\n3. Third";
        let n = synthesize_plan_nodes(g);
        assert_eq!(n.len(), 3);
        assert_eq!(n[2].depends_on, vec!["n2".to_string()]);
    }
}
