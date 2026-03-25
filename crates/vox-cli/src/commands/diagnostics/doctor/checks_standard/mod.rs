//! Default `vox doctor` checks: optional test-health tools, then full toolchain audit.

mod tail;
mod test_health;
mod toolchain;

use super::common::Check;

pub async fn run_checks(auto_heal: bool, test_health: bool, checks: &mut Vec<Check>) {
    if test_health::run(test_health, checks).await {
        return;
    }
    toolchain::run(auto_heal, checks).await;
    tail::run(auto_heal, checks).await;
}
