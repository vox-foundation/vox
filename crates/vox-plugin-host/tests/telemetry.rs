use tracing_test::traced_test;
use vox_plugin_host::telemetry;

#[traced_test]
#[test]
fn discovered_event_includes_id_and_version() {
    telemetry::discovered("test-id", "1.2.3", "code", 1);
    assert!(logs_contain("plugin.discovered"));
    assert!(logs_contain("test-id"));
}
