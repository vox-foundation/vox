async fn charge_card(amount_cents: i64) -> Result<String, String> {
    async fn __vox_activity_body_charge_card(amount_cents: i64) -> Result<String, anyhow::Error> {
        println!(
            "{}",
            vec![vox_actor_runtime::builtins::vox_display(
                &serde_json::to_value(
                    &(format!(
                        "{}{}",
                        format!(
                            "{}{}",
                            "Charged ".to_string(),
                            vox_actor_runtime::builtins::vox_display(
                                &serde_json::to_value(&(amount_cents)).unwrap_or_default()
                            )
                        ),
                        " cents".to_string()
                    ))
                )
                .unwrap_or_default()
            )]
            .join(" ")
        );
        return Ok("tx_9f8a7b".to_string());
    }
    __vox_activity_body_charge_card(amount_cents)
        .await
        .map_err(|e| e.to_string())
}
async fn checkout_order(amount_cents: i64) -> Result<Result<String, String>, String> {
    let __vox_fn_hash: &'static str = "1b4bb10297fbb51938c54b52650489f12be8e56b261793ccc7c584a8f9820b8eede00b82dd4974fa341015534ade8b29f5f48fac26b31377fbbe430faae6a92b";
    let _ = __vox_fn_hash;
    let __vox_hir = ::vox_workflow_runtime::workflow::current_hir_module();
    let __vox_run_id: String = ::std::env::var("VOX_WORKFLOW_RUN_ID").unwrap_or_else(|_| {
        format!(
            "checkout_order-run-{}-{}",
            ::std::process::id(),
            ::std::time::SystemTime::now()
                .duration_since(::std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        )
    });
    let __vox_db_cfg = ::vox_db::DbConfig::resolve_canonical().map_err(|e| {
        format!(
            "vox durable workflow: resolve VoxDb config (VOX_DB_URL+TOKEN, or VOX_DB_PATH): {e}"
        )
    })?;
    let __vox_db = ::std::sync::Arc::new(
        ::vox_db::VoxDb::connect(__vox_db_cfg)
            .await
            .map_err(|e| format!("vox durable workflow: connect VoxDb: {e}"))?,
    );
    let mut __vox_tracker = ::vox_workflow_runtime::VoxDbTracker::new(__vox_db, __vox_run_id);
    let __vox_args: ::std::vec::Vec<::serde_json::Value> =
        ::std::vec![::serde_json::to_value(&amount_cents).unwrap_or(::serde_json::Value::Null),];
    let __vox_journal = ::vox_workflow_runtime::workflow::interpret_workflow_durable(
        __vox_hir,
        "checkout_order",
        __vox_args,
        &mut __vox_tracker,
    )
    .await
    .map_err(|e| e.to_string())?;
    let __vox_debug_dispatch = ::std::env::var("VOX_RUN_WORKFLOW")
        .ok()
        .as_deref()
        .map(str::trim)
        == Some("checkout_order");
    if __vox_debug_dispatch {
        println!(
            "{}",
            ::serde_json::to_string(&__vox_journal).map_err(|e| e.to_string())?
        );
    }
    let __vox_completed = __vox_journal.iter().rev().any(|__e| {
        __e.get("event").and_then(::serde_json::Value::as_str) == Some("WorkflowCompleted")
    });
    if !__vox_completed {
        if __vox_debug_dispatch {
            ::std::process::exit(0);
        }
        return Err(
            "vox durable workflow: run parked (no WorkflowCompleted); resume with the same VOX_WORKFLOW_RUN_ID"
                .to_string(),
        );
    }
    Ok(
        ::vox_workflow_runtime::workflow::extract_terminal_return::<Result<String, String>>(
            &__vox_journal,
        )
        .map_err(|e| e.to_string())?,
    )
}
fn main() -> Result<String, String> {
    return checkout_order(4900i64);
}
/// Runs a workflow by name with positional JSON args (`VOX_RUN_WORKFLOW` / `VOX_WORKFLOW_ARGS`).
pub async fn __vox_run_workflow(
    __vox_wf_name: &str,
    __vox_wf_args: &[serde_json::Value],
) -> Result<(), String> {
    match __vox_wf_name {
        "checkout_order" => {
            let amount_cents = serde_json::from_value::<i64>(
                __vox_wf_args
                    .get(0)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )
            .map_err(|e| format!("workflow checkout_order arg 0 (amount_cents): {e}"))?;
            let out = checkout_order(amount_cents).await?;
            match out {
                Ok(_) => Ok(()),
                Err(_) => Err("workflow checkout_order returned Err".to_string()),
            }
        }
        _ => Err(format!("unknown workflow: {__vox_wf_name}")),
    }
}
pub fn run_checkout(amount_cents: i64) -> Result<String, String> {
    return checkout_order(amount_cents);
}
