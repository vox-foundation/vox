use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AxSnapshot {
    pub page_id: String,
    pub url: String,
    pub title: String,
    pub tree: String,
    pub refs: BTreeMap<String, AxRef>,
    pub truncated: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AxRef {
    pub ref_id: String,
    pub role: String,
    pub name: String,
    pub backend_dom_node_id: i64,
    pub sensitive: bool,
    pub box_css: Option<AxBox>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AxBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    pub interactive_only: bool,
    pub max_depth: u32,
    pub max_nodes: u32,
    pub include_boxes: bool,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            interactive_only: true,
            max_depth: 12,
            max_nodes: 80,
            include_boxes: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompactAx {
    pub tree: String,
    pub refs: BTreeMap<String, AxRef>,
    pub truncated: bool,
}

pub fn compact_ax_snapshot(nodes: &[serde_json::Value], opts: &SnapshotOptions) -> CompactAx {
    let by_id: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.get("nodeId")?.as_str().map(|id| (id, index)))
        .collect();
    let child_ids: HashSet<&str> = nodes
        .iter()
        .flat_map(|node| {
            node.get("childIds")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
        })
        .collect();
    let roots: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            let is_child = node
                .get("nodeId")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| child_ids.contains(id));
            (!is_child).then_some(index)
        })
        .collect();

    let mut state = CompactState {
        opts,
        nodes,
        by_id,
        lines: Vec::new(),
        refs: BTreeMap::new(),
        visited: HashSet::new(),
        truncated: false,
    };
    for index in roots {
        if state.visit(index, 0) {
            break;
        }
    }

    CompactAx {
        tree: state.lines.join("\n"),
        refs: state.refs,
        truncated: state.truncated,
    }
}

struct CompactState<'a> {
    opts: &'a SnapshotOptions,
    nodes: &'a [serde_json::Value],
    by_id: HashMap<&'a str, usize>,
    lines: Vec<String>,
    refs: BTreeMap<String, AxRef>,
    visited: HashSet<usize>,
    truncated: bool,
}

impl CompactState<'_> {
    /// Returns true when the node cap stops the traversal.
    fn visit(&mut self, index: usize, depth: u32) -> bool {
        if !self.visited.insert(index) {
            return false;
        }
        if depth > self.opts.max_depth {
            self.truncated = true;
            return false;
        }

        let node = &self.nodes[index];
        let role = ax_string(node.get("role"))
            .or_else(|| ax_string(node.get("chromeRole")))
            .unwrap_or_default();
        let normalized_role = role.to_ascii_lowercase();
        let interactive = is_interactive_role(&normalized_role);
        let ignored = node
            .get("ignored")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let emit = !ignored && (!self.opts.interactive_only || interactive);

        if emit {
            if self.lines.len() >= self.opts.max_nodes as usize {
                self.truncated = true;
                return true;
            }

            let name = ax_string(node.get("name")).unwrap_or_default();
            let backend_dom_node_id = node
                .get("backendDOMNodeId")
                .and_then(serde_json::Value::as_i64);
            let ref_id =
                interactive
                    .then_some(backend_dom_node_id)
                    .flatten()
                    .map(|backend_dom_node_id| {
                        let ref_id = format!("e{}", self.refs.len() + 1);
                        self.refs.insert(
                            ref_id.clone(),
                            AxRef {
                                ref_id: ref_id.clone(),
                                role: normalized_role.clone(),
                                name: name.clone(),
                                backend_dom_node_id,
                                sensitive: is_sensitive(node, &normalized_role, &name),
                                box_css: None,
                            },
                        );
                        ref_id
                    });

            let mut line = format!("{}- {role}", "  ".repeat(depth as usize));
            if !name.is_empty() {
                line.push(' ');
                line.push_str(&serde_json::to_string(&name).expect("string serialization"));
            }
            if let Some(ref_id) = ref_id {
                line.push_str(&format!(" [ref={ref_id}]"));
            }
            self.lines.push(line);
        }

        let child_depth = depth.saturating_add(1);
        let child_indexes: Vec<usize> = node
            .get("childIds")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .filter_map(|id| self.by_id.get(id).copied())
            .collect();
        for child_index in child_indexes {
            if self.visit(child_index, child_depth) {
                return true;
            }
        }
        false
    }
}

fn ax_string(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Object(value) => value
            .get("value")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        _ => None,
    }
}

fn is_interactive_role(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "link"
            | "textbox"
            | "searchbox"
            | "checkbox"
            | "radio"
            | "combobox"
            | "listbox"
            | "listitem"
            | "menuitem"
            | "tab"
            | "switch"
            | "slider"
            | "spinbutton"
    )
}

fn is_sensitive(node: &serde_json::Value, role: &str, name: &str) -> bool {
    ((role == "textbox" || role == "searchbox") && is_sensitive_name(name))
        || node
            .get("properties")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|properties| {
                properties.iter().any(|property| {
                    ax_string(property.get("name")).is_some_and(|name| {
                        name.eq_ignore_ascii_case("autocomplete")
                            && ax_string(property.get("value")).is_some_and(|value| {
                                value.to_ascii_lowercase().contains("password")
                            })
                    })
                })
            })
}

fn is_sensitive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words
        .iter()
        .any(|word| matches!(*word, "password" | "passwd" | "cvv" | "cvc"))
        || words
            .windows(2)
            .any(|pair| matches!(pair, ["card", "number"] | ["credit", "card"]))
        || words.iter().enumerate().any(|(index, word)| {
            *word == "pin" && words.get(index + 1..index + 3) != Some(&["your", "location"])
        })
}

/// Wrap untrusted page text with a nonce delimiter. This is not a MAC.
pub fn wrap_snapshot_tree(tree: &str) -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let raw = format!("{:x}{:x}", elapsed.as_secs(), elapsed.subsec_nanos());
    let nonce = if raw.len() >= 16 {
        raw[raw.len() - 16..].to_owned()
    } else {
        format!("{raw:0>16}")
    };
    format!("BEGIN_PAGE_SNAPSHOT nonce={nonce}\n{tree}\nEND_PAGE_SNAPSHOT nonce={nonce}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_options_default_is_interactive() {
        let d = SnapshotOptions::default();
        assert!(d.interactive_only);
        assert_eq!(d.max_depth, 12);
        assert_eq!(d.max_nodes, 80);
        assert!(!d.include_boxes);
    }

    #[test]
    fn compact_assigns_sequential_refs_to_interactive_roles() {
        let raw = include_str!("../tests/fixtures/ax_tree_button.json");
        let nodes: Vec<serde_json::Value> = serde_json::from_str(raw).unwrap();
        let out = compact_ax_snapshot(&nodes, &SnapshotOptions::default());
        assert!(out.tree.contains("[ref=e1]"));
        assert!(out.tree.contains("button \"Submit\""));
        assert!(!out.tree.contains("heading"));
        assert_eq!(out.refs["e1"].backend_dom_node_id, 11);
        assert!(!out.refs["e1"].sensitive);
    }

    #[test]
    fn skips_interactive_nodes_without_backend_id() {
        let nodes = serde_json::json!([
            {"nodeId":"1","role":{"type":"role","value":"button"},"name":{"type":"computedString","value":"Ghost"}}
        ]);
        let out = compact_ax_snapshot(nodes.as_array().unwrap(), &SnapshotOptions::default());
        assert!(out.refs.is_empty());
        assert!(!out.tree.contains("[ref="));
    }

    #[test]
    fn password_textbox_is_sensitive_pin_location_is_not() {
        let nodes = serde_json::json!([
            {"nodeId":"1","role":{"type":"role","value":"textbox"},"name":{"type":"computedString","value":"Password"},"backendDOMNodeId":4},
            {"nodeId":"2","role":{"type":"role","value":"textbox"},"name":{"type":"computedString","value":"PIN your location"},"backendDOMNodeId":5}
        ]);
        let out = compact_ax_snapshot(nodes.as_array().unwrap(), &SnapshotOptions::default());
        assert!(out.refs["e1"].sensitive);
        assert!(!out.refs["e2"].sensitive);
    }

    #[test]
    fn empty_ax_tree_is_empty_not_truncated() {
        let out = compact_ax_snapshot(&[], &SnapshotOptions::default());
        assert!(out.tree.is_empty() || out.tree == "- RootWebArea");
        assert!(out.refs.is_empty());
        assert!(!out.truncated);
    }

    #[test]
    fn max_nodes_sets_truncated() {
        let mut nodes = vec![
            serde_json::json!({"nodeId":"0","role":{"value":"RootWebArea"},"childIds":["1","2","3"]}),
        ];
        for i in 1..=3 {
            nodes.push(serde_json::json!({
                "nodeId": i.to_string(),
                "role": {"value":"button"},
                "name": {"value": format!("B{i}")},
                "backendDOMNodeId": i
            }));
        }
        let opts = SnapshotOptions {
            max_nodes: 1,
            ..SnapshotOptions::default()
        };
        let out = compact_ax_snapshot(&nodes, &opts);
        assert!(out.truncated);
        assert_eq!(out.refs.len(), 1);
    }

    #[test]
    fn wrap_snapshot_tree_begin_equals_end_nonce() {
        let wrapped = wrap_snapshot_tree("- button \"Go\" [ref=e1]");
        let begin = wrapped.lines().next().unwrap();
        let end = wrapped.lines().last().unwrap();
        let nonce = begin.strip_prefix("BEGIN_PAGE_SNAPSHOT nonce=").unwrap();
        assert_eq!(end, &format!("END_PAGE_SNAPSHOT nonce={nonce}"));
        assert_eq!(nonce.len(), 16);
    }
}
