//! Multi-turn conversation templates.

/// Return follow-up instruction templates for a given construct.
/// These are used after a base instruction pair to simulate multi-turn refinement.
/// `{name}` is replaced with the extracted identifier.
pub fn followup_templates(construct: &str) -> &[&str] {
    match construct {
        "function" => &[
            "Now add error handling that returns a Result type",
            "Add a docstring explaining what the {name} function does",
            "Refactor {name} to accept its parameters as a struct",
        ],
        "component" => &[
            "Add a loading state and error boundary to {name}",
            "Extract the {name} component props into a typed struct",
            "Add responsive CSS classes to the {name} component",
        ],
        "actor" => &[
            "Add history tracking to the {name} actor so it records all state changes",
            "Add a reset() handler to {name} that restores the initial state",
            "Add a subscription mechanism to notify observers of {name} state changes",
        ],
        "workflow" => &[
            "Add compensating transactions to the {name} workflow for rollback on failure",
            "Add exponential backoff to the retry policy in {name}",
            "Extract the activities in {name} into separate named functions",
        ],
        "activity" => &[
            "Add idempotency checking to {name} so it can safely be retried",
            "Add structured logging to {name} using Vox's tracing primitives",
        ],
        "table" => &[
            "Add an index on the most frequently queried field in {name}",
            "Add created_at and updated_at timestamp fields to {name}",
        ],
        "query" => &[
            "Add pagination (limit and offset) parameters to {name}",
            "Add filtering by status field to {name}",
        ],
        "test" => &[
            "Add edge case tests for null/empty inputs to {name}",
            "Add property-based tests to verify {name} handles all input ranges",
        ],
        "mcp_tool" => &[
            "Add input validation and descriptive error messages to {name}",
            "Add rate limiting to {name} to prevent abuse",
        ],
        _ => &[
            "Add error handling to the previous code",
            "Add a comment explaining the purpose of the previous code",
        ],
    }
}

/// Build multi-turn conversation pairs from a base (instruction, code) pair.
/// Returns a Vec of (follow_up_prompt, refined_code) where refined_code is the same
/// base code since we don't have a compiler that can apply the follow-up automatically —
/// the model should learn the *pattern* of refinement instructions.
pub fn generate_multiturn_pairs(
    construct: &str,
    name: &str,
    base_instruction: &str,
    code: &str,
    schema_version: &str,
    source: &str,
) -> Vec<serde_json::Value> {
    let mut pairs = Vec::new();
    let templates = followup_templates(construct);
    for template in templates.iter().take(2) {
        let follow_up = template.replace("{name}", name);
        // Multi-turn format: include the previous exchange as context in the prompt
        let prompt = format!(
            "Previous instruction: {base_instruction}\nPrevious code:\n```vox\n{code}\n```\n\nFollow-up: {follow_up}"
        );
        pairs.push(serde_json::json!({
            "prompt": prompt,
            "response": code,
            "instruction": follow_up,
            "output": code,
            "category": construct,
            "difficulty": crate::training::construct_difficulty(construct),
            "source": source,
            "rating": 4,
            "turn": 2,
            "schema_version": schema_version,
        }));
    }
    pairs
}
