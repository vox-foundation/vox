pub mod agent_loop;
mod conversation;
mod harness_issue_judge;
mod harness_issue_scorer;
mod history;
pub(crate) mod hydrate;
pub mod mentions;
pub mod mermaid_repair;
pub(crate) mod message;
pub mod research_intent;
pub mod research_turn;
pub mod system_prompt_vars;
mod turn_events;

pub use history::{
    chat_compact_up_to, chat_edit_message_in_place, chat_history, chat_queue_steer,
    chat_regenerate, chat_stream_cancel, chat_stream_resume,
};
pub use message::{chat_message, effective_model_pref};
