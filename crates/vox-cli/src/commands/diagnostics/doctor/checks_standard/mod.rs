//! Default `vox doctor` checks: optional test-health tools, then full toolchain audit.

mod tail;
mod test_health;
mod toolchain;
mod web_frontend;
mod gpu_hardware;

use super::common::Check;

pub async fn run_checks(auto_heal: bool, test_health: bool, checks: &mut Vec<Check>) {
    if test_health::run(test_health, checks).await {
        return;
    }
    toolchain::run(auto_heal, checks).await;
    gpu_hardware::run(checks).await;
    web_frontend::run(checks).await;
    tail::run(auto_heal, checks).await;
}
