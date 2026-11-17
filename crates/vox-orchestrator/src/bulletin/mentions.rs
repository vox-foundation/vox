//! Channel normalization, `@mention` token extraction, and doorbell helpers for [`BulletinBoard`].

use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::sync::Notify;

use super::BulletinBoard;

impl BulletinBoard {
    /// Phase 454 (CONC-02): Current monotonic `#seq` watermark on the Shared Sequence Board.
    #[must_use]
    pub fn head_seq(&self) -> u64 {
        self.concilium.seq_gen.load(Ordering::SeqCst)
    }

    /// Phase 454 (CONC-02): Normalize a `#channel` name to lowercase with a leading `#`.
    #[must_use]
    pub fn normalize_channel(channel: &str) -> String {
        let t = channel.trim();
        if t.is_empty() {
            "#main".to_string()
        } else if t.starts_with('#') {
            t.to_ascii_lowercase()
        } else {
            format!("#{}", t.to_ascii_lowercase())
        }
    }

    /// Phase 454 (CONC-02): Extract deduplicated `@agent`, `@all`, and `@#channel` mentions.
    #[must_use]
    pub fn extract_mentions(text: &str) -> Vec<String> {
        let bytes = text.as_bytes();
        let (mut out, mut i) = (Vec::new(), 0);
        while i < bytes.len() {
            if bytes[i] == b'@' && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
                let start = i + 1;
                let mut end = start + usize::from(start < bytes.len() && bytes[start] == b'#');
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric()
                        || matches!(bytes[end], b'_' | b'-' | b'.'))
                {
                    end += 1;
                }
                let raw = text[start..end].trim_end_matches(['.', '-', '_']);
                if !raw.is_empty() && raw != "#" {
                    let m = format!("@{raw}");
                    if !out.iter().any(|x: &String| x.eq_ignore_ascii_case(&m)) {
                        out.push(m);
                    }
                }
                i = end;
            } else {
                i += 1;
            }
        }
        out
    }

    /// Phase 454 (CONC-02): Return or register the `tokio::sync::Notify` doorbell for `agent_id`.
    #[must_use]
    pub fn doorbell_for(&self, agent_id: &str) -> Arc<Notify> {
        let key = agent_id.trim().trim_start_matches('@').to_string();
        self.concilium
            .doorbells
            .write()
            .entry(key)
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_orchestrator_types::concilium::{
        ComputeTier, ConciliumPilotActionInput, ConciliumPilotActionKind, SequenceBoardFilter,
        SwarmWorkspaceMode,
    };

    #[tokio::test]
    async fn monotonic_seq_park_wakeup_and_pilot_actions() {
        let board = BulletinBoard::new(16);
        let notify = board.doorbell_for("verifier");
        assert_eq!(
            board.park_worker("verifier", "verifier", Some("#verify")),
            0
        );
        assert_eq!(board.concilium_snapshot().parked_workers, 1);

        let e1 = board.post_board_entry(
            "parser",
            "author",
            "landed parser slice, cc @#verify and @verifier",
            ComputeTier::LocalMensGpu,
            SwarmWorkspaceMode::Share,
            Some("crates/vox-parser/".into()),
            None,
            Some("DIFF +42/-3".into()),
            false,
        );
        assert_eq!((e1.seq, e1.channel.as_str()), (1, "#parser"));
        assert_eq!(
            e1.mentions,
            vec!["@#verify".to_string(), "@verifier".to_string()]
        );
        tokio::time::timeout(vox_config::timeouts::D_100MS, notify.notified())
            .await
            .expect("wake");

        let mut input = ConciliumPilotActionInput {
            action: ConciliumPilotActionKind::ForkAgent,
            agent_id: "verifier".into(),
            channel: Some("#verify".into()),
            directive: None,
            fork_name: Some("verifier-b".into()),
            workspace_mode: Some(SwarmWorkspaceMode::Share),
            target_tier: None,
            target_node_id: None,
            target_model_id: None,
        };
        assert_eq!(
            board.apply_pilot_action(&input).forked_agent_id.as_deref(),
            Some("verifier-b")
        );

        input.action = ConciliumPilotActionKind::SwapAgentCompute;
        input.agent_id = "verifier-b".into();
        input.target_tier = Some(ComputeTier::PopuliQuicPeer);
        assert!(board.apply_pilot_action(&input).ok);
        let entries = board.read_board(&SequenceBoardFilter {
            since_seq: Some(1),
            channel: Some("#verify".into()),
            ..Default::default()
        });
        assert_eq!(entries.len(), 2);
    }
}
