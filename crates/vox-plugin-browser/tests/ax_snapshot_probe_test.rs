use serde_json::json;

#[test]
fn test_compact_ax_probe_bounds() {
    let fixture = [
        json!({ "role": { "type": "role", "value": "button" }, "name": { "value": "Submit" } }),
        json!({ "role": { "type": "role", "value": "paragraph" }, "name": { "value": "Static text" } }),
    ];
    let is_interactive = |r: &str| matches!(r, "button" | "link" | "textbox");
    let interactive_count = fixture
        .iter()
        .filter(|n| {
            let role = n["role"]["value"].as_str().unwrap_or("");
            is_interactive(role)
        })
        .count();
    assert_eq!(interactive_count, 1);
}
