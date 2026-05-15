//! Integration: ProducerRegistry composes and dedups output across producers.

use async_trait::async_trait;
use vox_research_events::ResearchEvent;
use vox_scientia_producers::{Producer, ProducerContext, ProducerRegistry};

struct OneShotProducer(&'static str, &'static str);

#[async_trait]
impl Producer for OneShotProducer {
    fn name(&self) -> &'static str {
        self.0
    }
    async fn observe(&self, _ctx: &ProducerContext) -> Vec<ResearchEvent> {
        vec![ResearchEvent::FindingCandidateProposed {
            finding_id: self.1.into(),
            claim_ids: vec![],
            worthiness_score: 0.5,
            session_id: "smoke".into(),
        }]
    }
}

#[tokio::test]
async fn registry_runs_all_producers_in_order() {
    let mut reg = ProducerRegistry::new();
    reg.register(Box::new(OneShotProducer("a", "fid-a")));
    reg.register(Box::new(OneShotProducer("b", "fid-b")));
    assert_eq!(reg.len(), 2);
    assert_eq!(reg.producer_names(), vec!["a", "b"]);

    let ctx = ProducerContext::for_test();
    let events = reg.run_all(&ctx).await;
    assert_eq!(events.len(), 2);
    match &events[0] {
        ResearchEvent::FindingCandidateProposed { finding_id, .. } => {
            assert_eq!(finding_id, "fid-a");
        }
        _ => panic!("expected FindingCandidateProposed"),
    }
}

#[tokio::test]
async fn registry_dedups_collisions_across_producers() {
    let mut reg = ProducerRegistry::new();
    reg.register(Box::new(OneShotProducer("a", "shared")));
    reg.register(Box::new(OneShotProducer("b", "shared")));
    let ctx = ProducerContext::for_test();
    let events = reg.run_all(&ctx).await;
    assert_eq!(events.len(), 1, "duplicate finding_id must collapse");
}

#[tokio::test]
async fn empty_registry_yields_no_events() {
    let reg = ProducerRegistry::new();
    assert!(reg.is_empty());
    let ctx = ProducerContext::for_test();
    let events = reg.run_all(&ctx).await;
    assert!(events.is_empty());
}
