//! `examples/mesh/node-capabilities.sample.json` must deserialize as [`vox_mesh::NodeRecord`].

use vox_mesh::NodeRecord;

const SAMPLE: &str = include_str!("node-capabilities.sample.json");

#[test]
fn sample_json_roundtrips_node_record() {
    let n: NodeRecord = serde_json::from_str(SAMPLE).expect("sample JSON parses as NodeRecord");
    assert_eq!(n.id, "edge-android-01");
    assert!(n.capabilities.gpu_vulkan);
    assert_eq!(n.scope_id.as_deref(), Some("compose-demo"));
}
