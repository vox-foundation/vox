use vox_orchestrator::events::{AgentEvent, AgentEventKind, EventId};
use vox_orchestrator::types::AgentId;
use vox_terminal_core::agent::{AgentAdapterConfig, translate_event};
use vox_terminal_core::session::SessionEvent;

fn fake_agent_id() -> AgentId {
    AgentId(42)
}

#[test]
fn token_streamed_maps_to_agent_message() {
    let ev = AgentEvent {
        id: EventId(1),
        timestamp_ms: 0,
        kind: AgentEventKind::TokenStreamed {
            agent_id: fake_agent_id(),
            text: "hello world".into(),
            session_id: None,
        },
    };
    let cfg = AgentAdapterConfig { agent_id: None };
    let translated = translate_event(&ev, &cfg);
    assert!(
        matches!(&translated, Some(SessionEvent::AgentMessage { text }) if text == "hello world"),
        "got: {translated:?}"
    );
}

#[test]
fn non_token_events_return_none() {
    let ev = AgentEvent {
        id: EventId(2),
        timestamp_ms: 0,
        kind: AgentEventKind::AgentRetired {
            agent_id: fake_agent_id(),
        },
    };
    let cfg = AgentAdapterConfig { agent_id: None };
    assert!(translate_event(&ev, &cfg).is_none());
}

#[test]
fn route_agent_prompt_classifies_tasks_and_axes_intelligently() {
    use vox_terminal_core::agent::route_agent_prompt;

    let debug = route_agent_prompt("fix the failing cargo test with exit code 101");
    assert_eq!(debug.task, "Debugging");
    assert!(
        debug.precision_axis >= 70,
        "debug should be precision-weighted: {debug:?}"
    );
    assert!(!debug.model_id.is_empty());

    let plan = route_agent_prompt("architect a multi-node terminal session roadmap");
    assert_eq!(plan.task, "Planning");
    assert_eq!(plan.complexity, 8);
    assert!(!plan.model_id.is_empty());

    let parse = route_agent_prompt("summarize and classify this log line");
    assert_eq!(parse.task, "Parsing");
    assert!(
        parse.efficiency_axis >= 70,
        "parsing should be cost-first: {parse:?}"
    );

    let research = route_agent_prompt("why does WebGL outperform DOM canvas rendering");
    assert_eq!(research.task, "Research");
    assert!(research.summary().contains("task=Research"));
}
