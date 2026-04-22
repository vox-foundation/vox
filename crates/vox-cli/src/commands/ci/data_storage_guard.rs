use anyhow::Result;

use super::cmd_enums::GuardOpts;

#[derive(Debug, Clone, serde::Serialize)]
pub struct GuardReport {
    // Authored in M-04
}

impl GuardReport {
    pub fn empty() -> Self {
        Self {}
    }
}

pub fn run(_opts: &GuardOpts) -> Result<GuardReport> {
    println!("stub");
    Ok(GuardReport::empty())
}
