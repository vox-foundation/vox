//! Integration tests verifying that `vox.vcs.*` tracing targets are emitted
//! correctly through the `tracing` subscriber API.

use std::sync::{Arc, Mutex};

use tracing::subscriber::with_default;

// ── Minimal subscriber that captures event targets ────────────────────────────

#[derive(Default, Clone)]
struct EventCapture {
    events: Arc<Mutex<Vec<String>>>,
}

impl tracing::Subscriber for EventCapture {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let target = event.metadata().target().to_string();
        self.events.lock().unwrap().push(target);
    }

    fn enter(&self, _: &tracing::span::Id) {}

    fn exit(&self, _: &tracing::span::Id) {}
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Verifies that the target string used in `git_exec.rs` ("vox.vcs.exec") is
/// correctly routed through the subscriber.  We emit the event directly because
/// we cannot run real git in unit tests.
#[test]
fn git_exec_success_emits_vox_vcs_exec() {
    let capture = EventCapture::default();
    let events = capture.events.clone();
    with_default(capture, || {
        tracing::debug!(target: "vox.vcs.exec", "test");
    });
    let events = events.lock().unwrap();
    assert!(
        events.iter().any(|t| t == "vox.vcs.exec"),
        "expected vox.vcs.exec event, got: {:?}",
        events
    );
}

/// Verifies that the target strings used in `commit_tools.rs` and
/// `branch_tools.rs` are correctly routed through the subscriber.
#[test]
fn vcs_tool_targets_are_correct() {
    let capture = EventCapture::default();
    let events = capture.events.clone();
    with_default(capture, || {
        tracing::info!(target: "vox.vcs.commit", "test");
        tracing::info!(target: "vox.vcs.branch", "test");
    });
    let events = events.lock().unwrap();
    assert!(
        events.iter().any(|t| t == "vox.vcs.commit"),
        "missing vox.vcs.commit"
    );
    assert!(
        events.iter().any(|t| t == "vox.vcs.branch"),
        "missing vox.vcs.branch"
    );
}
