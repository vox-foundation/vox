use crate::commands::diagnostics::doctor::common::Check;

pub async fn run(checks: &mut Vec<Check>) {
    checks.push(Check {
        name: "GPU Discovery".to_string(),
        pass: true,
        detail: "Delegated to vox-mens (run `vox mens probe`)".to_string(),
    });
}
