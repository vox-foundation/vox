use crate::types::{
    AgentId, A2AMessage, A2AMessageType, MessagePriority, ThreadId, VcsContext,
};

use super::MessageBus;

#[test]
fn message_ids_strictly_increasing_for_correlation() {
    let bus = MessageBus::new(10);
    let a1 = AgentId(1);
    let a2 = AgentId(2);
    bus.register_agent(a1);
    bus.register_agent(a2);
    let id1 = bus.send(a1, a2, A2AMessageType::FreeForm, "a");
    let id2 = bus.send(a1, a2, A2AMessageType::FreeForm, "b");
    assert!(
        id2.0 > id1.0,
        "monotonic ids support delivery correlation / dedup policies"
    );
}

#[test]
fn send_and_receive() {
    let bus = MessageBus::new(100);
    let a1 = AgentId(1);
    let a2 = AgentId(2);

    bus.register_agent(a1);
    bus.register_agent(a2);

    let id = bus.send(a1, a2, A2AMessageType::ProgressUpdate, "50% done");

    assert_eq!(bus.unread_count(a2), 1);
    assert_eq!(bus.unread_count(a1), 0);

    let inbox = bus.inbox(a2);
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].id, id);
    assert_eq!(inbox[0].payload, "50% done");
}

#[test]
fn broadcast_reaches_all() {
    let bus = MessageBus::new(100);
    let a1 = AgentId(1);
    let a2 = AgentId(2);
    let a3 = AgentId(3);

    bus.register_agent(a1);
    bus.register_agent(a2);
    bus.register_agent(a3);

    bus.broadcast(a1, A2AMessageType::CompletionNotice, "Task done");

    assert_eq!(bus.unread_count(a1), 0);
    assert_eq!(bus.unread_count(a2), 1);
    assert_eq!(bus.unread_count(a3), 1);
}

#[test]
fn acknowledge_marks_read() {
    let bus = MessageBus::new(100);
    let a1 = AgentId(1);
    let a2 = AgentId(2);

    bus.register_agent(a1);
    bus.register_agent(a2);

    let id = bus.send(a1, a2, A2AMessageType::HelpRequest, "Need help");
    assert_eq!(bus.unread_count(a2), 1);

    bus.acknowledge(a2, id);
    assert_eq!(bus.unread_count(a2), 0);
    assert_eq!(bus.inbox_all(a2).len(), 1);
}

#[test]
fn audit_trail() {
    let bus = MessageBus::new(100);
    let a1 = AgentId(1);
    let a2 = AgentId(2);

    bus.register_agent(a1);
    bus.register_agent(a2);

    bus.send(a1, a2, A2AMessageType::FreeForm, "hello");
    bus.send(a2, a1, A2AMessageType::FreeForm, "hi back");

    assert_eq!(bus.total_messages(), 2);
    assert_eq!(bus.audit_trail().len(), 2);
}

#[test]
fn inbox_overflow() {
    let bus = MessageBus::new(2);
    let a1 = AgentId(1);
    let a2 = AgentId(2);

    bus.register_agent(a1);
    bus.register_agent(a2);

    bus.send(a1, a2, A2AMessageType::FreeForm, "msg1");
    bus.send(a1, a2, A2AMessageType::FreeForm, "msg2");
    bus.send(a1, a2, A2AMessageType::FreeForm, "msg3");

    let inbox = bus.inbox_all(a2);
    assert_eq!(inbox.len(), 2);
    assert_eq!(inbox[0].payload, "msg2");
}

#[test]
fn priority_sorted_inbox() {
    let bus = MessageBus::new(100);
    let a1 = AgentId(1);
    let a2 = AgentId(2);
    bus.register_agent(a1);
    bus.register_agent(a2);

    let id_low = bus.next_id();
    let low_msg = A2AMessage::new(id_low, a1, Some(a2), A2AMessageType::FreeForm, "low")
        .with_priority(MessagePriority::Low);
    {
        let mut inboxes = crate::sync_lock::rw_write(&bus.inboxes);
        let inbox_lock = inboxes
            .entry(a2)
            .or_insert_with(|| std::sync::RwLock::new(std::collections::VecDeque::new()));
        crate::sync_lock::rw_write(inbox_lock).push_back(low_msg.clone());
    }
    crate::sync_lock::rw_write(&bus.audit_trail).push(low_msg);

    let id_crit = bus.next_id();
    let crit_msg = A2AMessage::new(
        id_crit,
        a1,
        Some(a2),
        A2AMessageType::ErrorReport,
        "critical!",
    )
    .with_priority(MessagePriority::Critical);
    {
        let mut inboxes = crate::sync_lock::rw_write(&bus.inboxes);
        let inbox_lock = inboxes
            .entry(a2)
            .or_insert_with(|| std::sync::RwLock::new(std::collections::VecDeque::new()));
        crate::sync_lock::rw_write(inbox_lock).push_back(crit_msg.clone());
    }
    crate::sync_lock::rw_write(&bus.audit_trail).push(crit_msg);

    let inbox = bus.inbox(a2);
    assert_eq!(inbox.len(), 2);
    assert_eq!(inbox[0].priority, MessagePriority::Critical);
    assert_eq!(inbox[1].priority, MessagePriority::Low);
}

#[test]
fn thread_message_grouping() {
    let bus = MessageBus::new(100);
    let a1 = AgentId(1);
    let a2 = AgentId(2);
    bus.register_agent(a1);
    bus.register_agent(a2);

    let thread = ThreadId::from("thread-abc");
    let ctx = VcsContext {
        snapshot_before: Some(1),
        snapshot_after: Some(2),
        touched_paths: vec!["src/parser.rs".parse().unwrap()],
        change_id: None,
        op_id: None,
        content_hash: None,
    };

    bus.send_with_vcs_context(
        a1,
        a2,
        A2AMessageType::ConflictDetected,
        "merge conflict on parser",
        ctx,
        MessagePriority::High,
        Some(thread.clone()),
    );
    bus.send(a1, a2, A2AMessageType::FreeForm, "unrelated");

    let threaded = bus.messages_in_thread(&thread);
    assert_eq!(threaded.len(), 1);
    assert!(threaded[0].vcs_context.is_some());
    let ctx_back = threaded[0].vcs_context.as_ref().unwrap();
    assert_eq!(ctx_back.snapshot_before, Some(1));
}

#[test]
fn conflict_notice_is_critical_priority() {
    let bus = MessageBus::new(100);
    let a1 = AgentId(1);
    let a2 = AgentId(2);
    bus.register_agent(a1);
    bus.register_agent(a2);

    bus.send_conflict_notice(a1, a2, "src/lib.rs", Some(42));
    let inbox = bus.inbox(a2);
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].msg_type, A2AMessageType::ConflictDetected);
    assert_eq!(inbox[0].priority, MessagePriority::Critical);
}
