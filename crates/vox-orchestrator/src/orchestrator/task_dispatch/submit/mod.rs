use std::time::Duration;

pub(super) const AGENT_NOTIFY_TIMEOUT: Duration = Duration::from_secs(30);

mod attention_fields;
mod batch;
mod goal;
mod task_submit;
