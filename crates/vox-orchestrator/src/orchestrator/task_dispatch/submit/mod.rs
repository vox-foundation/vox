use std::time::Duration;

#[cfg(feature = "runtime")]
pub(super) const AGENT_NOTIFY_TIMEOUT: Duration = Duration::from_secs(30);

mod attention_fields;
mod batch;
pub(crate) mod dei_plan_materialize;
mod goal;
mod task_submit;
