//! Default `vox doctor` checks: optional test-health tools, then full toolchain audit.

mod clavis;
mod gpu_hardware;
mod tail;
mod test_health;
mod toolchain;
mod vox_ignore;
mod web_frontend;

use super::common::Check;

pub async fn run_checks(auto_heal: bool, test_health: bool, checks: &mut Vec<Check>) {
    if test_health::run(test_health, checks).await {
        return;
    }
    toolchain::run(auto_heal, checks).await;
    clavis::run(auto_heal, checks).await;
    gpu_hardware::run(checks).await;
    vox_ignore::run(auto_heal, checks).await;
    web_frontend::run(checks).await;
    tail::run(auto_heal, checks).await;
}
