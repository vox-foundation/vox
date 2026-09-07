//! Durability interception for workflow execution.
//!
//! Three families: activity Begin/End (after callee eval), Patch
//! (`HirExpr::WorkflowVersion`), and host builtins (`workflow_wait` /
//! `workflow_wait_signal`). `with { }` stashes options for the next Begin.

use crate::ast::span::Span;
use crate::eval::value::VoxValue;
use crate::eval::{EvalError, Interpreter};
use crate::hir::nodes::{HirExpr, HirModule, HirStmt};
use std::collections::HashMap;

/// Stamp durable call-site ordinals without widening [`HirExpr::Call`].
///
/// Ordinals restart for each function and callee, so inserting a call to a
/// different activity does not rename an existing activity's identity.
pub fn stamp_call_ordinals(hir: &HirModule) -> HashMap<Span, u32> {
    let mut ordinals = HashMap::new();
    for function in &hir.functions {
        let mut next_by_callee = HashMap::<String, u32>::new();
        for statement in &function.body {
            stamp_stmt(statement, &mut next_by_callee, &mut ordinals);
        }
    }
    ordinals
}

fn stamp_stmt(
    stmt: &HirStmt,
    next_by_callee: &mut HashMap<String, u32>,
    ordinals: &mut HashMap<Span, u32>,
) {
    match stmt {
        HirStmt::Expr { expr, .. } | HirStmt::Let { value: expr, .. } => {
            stamp_expr(expr, next_by_callee, ordinals);
        }
        HirStmt::Assign { target, value, .. } => {
            stamp_expr(target, next_by_callee, ordinals);
            stamp_expr(value, next_by_callee, ordinals);
        }
        HirStmt::Return { value, .. } => {
            if let Some(expr) = value {
                stamp_expr(expr, next_by_callee, ordinals);
            }
        }
        HirStmt::While {
            condition, body, ..
        } => {
            stamp_expr(condition, next_by_callee, ordinals);
            for statement in body {
                stamp_stmt(statement, next_by_callee, ordinals);
            }
        }
        HirStmt::Loop { body, .. } => {
            for statement in body {
                stamp_stmt(statement, next_by_callee, ordinals);
            }
        }
        HirStmt::Break { .. } | HirStmt::Continue { .. } => {}
    }
}

fn stamp_expr(
    expr: &HirExpr,
    next_by_callee: &mut HashMap<String, u32>,
    ordinals: &mut HashMap<Span, u32>,
) {
    match expr {
        HirExpr::Call(callee, args, _, span) => {
            let key = match callee.as_ref() {
                HirExpr::Ident(name, _) => name.clone(),
                _ => "<dynamic>".to_string(),
            };
            let ordinal = next_by_callee.entry(key).or_default();
            ordinals.insert(*span, *ordinal);
            *ordinal = ordinal.saturating_add(1);
            stamp_expr(callee, next_by_callee, ordinals);
            for arg in args {
                stamp_expr(&arg.value, next_by_callee, ordinals);
            }
        }
        HirExpr::WorkflowVersion(version) => {
            let ordinal = next_by_callee
                .entry("workflow.version".to_string())
                .or_default();
            ordinals.insert(version.span, *ordinal);
            *ordinal = ordinal.saturating_add(1);
        }
        HirExpr::MethodCall(receiver, _, args, plan, _) => {
            stamp_expr(receiver, next_by_callee, ordinals);
            for arg in args {
                stamp_expr(&arg.value, next_by_callee, ordinals);
            }
            if let Some(plan) = plan {
                for arg in &plan.predicate_args {
                    stamp_expr(&arg.value, next_by_callee, ordinals);
                }
                if let Some(limit) = &plan.limit_value {
                    stamp_expr(limit, next_by_callee, ordinals);
                }
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, value) in fields {
                stamp_expr(value, next_by_callee, ordinals);
            }
        }
        HirExpr::ListLit(items, _)
        | HirExpr::TupleLit(items, _)
        | HirExpr::JsxFragment(items, _) => {
            for item in items {
                stamp_expr(item, next_by_callee, ordinals);
            }
        }
        HirExpr::Binary(_, left, right, _) | HirExpr::Index(left, right, _) => {
            stamp_expr(left, next_by_callee, ordinals);
            stamp_expr(right, next_by_callee, ordinals);
        }
        HirExpr::Unary(_, inner, _)
        | HirExpr::FieldAccess(inner, _, _)
        | HirExpr::Spawn(inner, _) => stamp_expr(inner, next_by_callee, ordinals),
        HirExpr::Match(subject, arms, _) => {
            stamp_expr(subject, next_by_callee, ordinals);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    stamp_expr(guard, next_by_callee, ordinals);
                }
                stamp_expr(&arm.body, next_by_callee, ordinals);
            }
        }
        HirExpr::If(condition, then_body, else_body, _) => {
            stamp_expr(condition, next_by_callee, ordinals);
            for statement in then_body {
                stamp_stmt(statement, next_by_callee, ordinals);
            }
            if let Some(body) = else_body {
                for statement in body {
                    stamp_stmt(statement, next_by_callee, ordinals);
                }
            }
        }
        HirExpr::For(_, _, iterable, body, key, _) => {
            stamp_expr(iterable, next_by_callee, ordinals);
            stamp_expr(body, next_by_callee, ordinals);
            if let Some(key) = key {
                stamp_expr(key, next_by_callee, ordinals);
            }
        }
        HirExpr::Lambda(params, _, body, _, _) => {
            for param in params {
                if let Some(default) = &param.default {
                    stamp_expr(default, next_by_callee, ordinals);
                }
            }
            stamp_expr(body, next_by_callee, ordinals);
        }
        HirExpr::With(inner, options, _) => {
            stamp_expr(inner, next_by_callee, ordinals);
            stamp_expr(options, next_by_callee, ordinals);
        }
        HirExpr::Block(statements, _) => {
            for statement in statements {
                stamp_stmt(statement, next_by_callee, ordinals);
            }
        }
        HirExpr::Try(try_expr) => stamp_expr(&try_expr.target, next_by_callee, ordinals),
        HirExpr::AsyncView(view) => {
            for arm in [
                &view.fetching_arm,
                &view.empty_arm,
                &view.error_arm,
                &view.ok_arm,
            ]
            .into_iter()
            .flatten()
            {
                stamp_expr(arm, next_by_callee, ordinals);
            }
        }
        HirExpr::Jsx(element) => {
            for attribute in &element.attributes {
                stamp_expr(&attribute.value, next_by_callee, ordinals);
            }
            for child in &element.children {
                stamp_expr(child, next_by_callee, ordinals);
            }
        }
        HirExpr::JsxSelfClosing(element) => {
            for attribute in &element.attributes {
                stamp_expr(&attribute.value, next_by_callee, ordinals);
            }
        }
        HirExpr::IntLit(..)
        | HirExpr::FloatLit(..)
        | HirExpr::StringLit(..)
        | HirExpr::BoolLit(..)
        | HirExpr::DecimalLit(..)
        | HirExpr::Ident(..) => {}
    }
}

/// One interception point. Parking is a decision on Wait / WaitSignal,
/// not a fake activity named `__durable_timer_wait`.
#[derive(Debug, Clone)]
pub enum HostCall {
    Begin {
        name: String,
        args: Vec<VoxValue>,
        options: Option<VoxValue>,
        span: Span,
        enclosing_fn: String,
        ordinal: u32,
        iteration: Vec<u32>,
    },
    End {
        name: String,
        result: VoxValue,
        /// True only when the body panicked or returned `_Panic`.
        failed: bool,
    },
    Patch {
        change_id: String,
        min: u32,
        max: u32,
    },
    Wait {
        deadline_ms: i64,
    },
    WaitSignal {
        key: String,
    },
}

/// Runner decision. There is no `Retry` — the runner re-sends `Execute`
/// after recording an attempt failure.
#[derive(Debug, Clone)]
pub enum HostDecision {
    Replay(VoxValue),
    Execute,
    Park(String),
    Ack,
}

/// Installed by the durable runner. Created on the interpreter thread;
/// do not add `+ Send`.
pub type ActivityHook = Box<dyn FnMut(HostCall) -> Result<HostDecision, EvalError>>;

fn take_hook(interp: &mut Interpreter) -> Option<ActivityHook> {
    interp.activity_hook.take()
}

fn restore_hook(interp: &mut Interpreter, hook: ActivityHook) {
    interp.activity_hook = Some(hook);
}

/// Intercept one activity call after the callee has evaluated to a named Fn.
pub(crate) fn dispatch_activity(
    interp: &mut Interpreter,
    name: &str,
    args: Vec<VoxValue>,
    span: Span,
) -> Result<VoxValue, EvalError> {
    let hook = match take_hook(interp) {
        Some(h) => h,
        None => return interp.call(name, args),
    };
    let mut hook = Some(hook);
    let options = interp.with_options.last().cloned();
    let ordinal = interp.call_ordinals.get(&span).copied().unwrap_or_default();
    let enclosing_fn = interp
        .enclosing_fn_stack
        .last()
        .cloned()
        .unwrap_or_default();
    let iteration = if interp.iteration_stack.is_empty() {
        vec![0]
    } else {
        interp.iteration_stack.clone()
    };
    let outcome = (|| -> Result<VoxValue, EvalError> {
        let decision = hook.as_mut().expect("hook is present")(HostCall::Begin {
            name: name.to_string(),
            args: args.clone(),
            options,
            span,
            enclosing_fn,
            ordinal,
            iteration,
        })?;
        match decision {
            HostDecision::Replay(value) => Ok(value),
            HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
            HostDecision::Execute => {
                restore_hook(interp, hook.take().expect("hook is present"));
                let result = match interp.call(name, args.clone()) {
                    Ok(v) => v,
                    Err(e) => {
                        hook = Some(take_hook(interp).ok_or_else(|| {
                            EvalError::Panic("activity hook lost during body execution".into())
                        })?);
                        let failed =
                            matches!(&e, EvalError::Panic(_) | EvalError::AssertionFailed(_));
                        let _ = hook.as_mut().expect("hook is present")(HostCall::End {
                            name: name.to_string(),
                            result: VoxValue::Null,
                            failed,
                        });
                        return Err(e);
                    }
                };
                hook = Some(take_hook(interp).ok_or_else(|| {
                    EvalError::Panic("activity hook lost during body execution".into())
                })?);
                let failed = matches!(&result, VoxValue::_Panic(_));
                match hook.as_mut().expect("hook is present")(HostCall::End {
                    name: name.to_string(),
                    result: result.clone(),
                    failed,
                })? {
                    HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
                    HostDecision::Replay(value) => Ok(value),
                    HostDecision::Ack | HostDecision::Execute => Ok(result),
                }
            }
            HostDecision::Ack => interp.call(name, args.clone()),
        }
    })();
    if let Some(hook) = hook {
        restore_hook(interp, hook);
    }
    outcome
}

pub(crate) fn dispatch_patch(
    interp: &mut Interpreter,
    change_id: &str,
    min: u32,
    max: u32,
) -> Result<VoxValue, EvalError> {
    let Some(mut hook) = take_hook(interp) else {
        return Ok(VoxValue::Null);
    };
    let decision = hook(HostCall::Patch {
        change_id: change_id.to_string(),
        min,
        max,
    });
    restore_hook(interp, hook);
    match decision? {
        HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
        _ => Ok(VoxValue::Null),
    }
}

pub(crate) fn dispatch_wait(
    interp: &mut Interpreter,
    deadline_ms: i64,
) -> Result<VoxValue, EvalError> {
    let Some(mut hook) = take_hook(interp) else {
        return Err(EvalError::Panic(
            "workflow_wait requires a durable runner".into(),
        ));
    };
    let decision = hook(HostCall::Wait { deadline_ms });
    restore_hook(interp, hook);
    match decision? {
        HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
        HostDecision::Ack | HostDecision::Execute | HostDecision::Replay(_) => Ok(VoxValue::Null),
    }
}

pub(crate) fn dispatch_wait_signal(
    interp: &mut Interpreter,
    key: String,
) -> Result<VoxValue, EvalError> {
    let Some(mut hook) = take_hook(interp) else {
        return Err(EvalError::Panic(
            "workflow_wait_signal requires a durable runner".into(),
        ));
    };
    let decision = hook(HostCall::WaitSignal { key });
    restore_hook(interp, hook);
    match decision? {
        HostDecision::Park(reason) => Err(EvalError::WorkflowParked(reason)),
        HostDecision::Replay(v) => Ok(v),
        HostDecision::Ack | HostDecision::Execute => Ok(VoxValue::Null),
    }
}

#[cfg(test)]
mod tests {
    use crate::eval::Interpreter;
    use crate::eval::activity_hook::{HostCall, HostDecision, stamp_call_ordinals};
    use crate::eval::value::VoxValue;
    use crate::hir::lower_module;
    use crate::lexer::cursor::lex;
    use crate::parser::parse;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn interp_for(src: &str) -> (Interpreter, std::collections::HashSet<String>) {
        let hir = lower_module(&parse(lex(src)).expect("parses"));
        let activities = hir
            .functions
            .iter()
            .filter(|f| f.durability == Some(crate::hir::nodes::DurabilityKind::Activity))
            .map(|f| f.name.clone())
            .collect();
        let mut interp = Interpreter::new(1_000_000);
        interp.run_module(&hir).expect("module loads");
        (interp, activities)
    }

    #[test]
    fn call_ordinals_are_scoped_by_function_and_callee() {
        const SRC: &str = r#"
activity old() to int { return 1 }
activity fresh() to int { return 2 }
workflow wf() to int {
    let a = fresh()
    let b = old()
    return old() + a + b
}
"#;
        let hir = lower_module(&parse(lex(SRC)).expect("parses"));
        let ordinals = stamp_call_ordinals(&hir);
        let wf = hir.functions.iter().find(|f| f.name == "wf").expect("wf");
        let spans: Vec<_> = wf
            .body
            .iter()
            .filter_map(|stmt| match stmt {
                crate::hir::nodes::HirStmt::Let {
                    value: crate::hir::nodes::HirExpr::Call(_, _, _, span),
                    ..
                } => Some(*span),
                _ => None,
            })
            .collect();
        assert_eq!(ordinals.get(&spans[0]), Some(&0));
        assert_eq!(ordinals.get(&spans[1]), Some(&0));
    }

    #[test]
    fn hook_observes_every_activity_call() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount * 2 }
workflow wf(amount: int) to int {
    let a = charge(amount)
    let b = charge(a)
    return b
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let seen_c = seen.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::Begin { name, .. } = &call {
                seen_c.borrow_mut().push(name.clone());
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                HostCall::End { .. } => HostDecision::Ack,
                _ => HostDecision::Ack,
            })
        }));
        let out = interp.call("wf", vec![VoxValue::Int(5)]).expect("runs");
        assert!(matches!(out, VoxValue::Int(20)), "5*2*2 = 20, got {out:?}");
        assert_eq!(
            *seen.borrow(),
            vec!["charge".to_string(), "charge".to_string()]
        );
    }

    #[test]
    fn intercept_after_callee_eval_not_only_ident() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount * 2 }
workflow wf(amount: int) to int {
    let f = charge
    return f(amount)
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let seen = Rc::new(RefCell::new(0usize));
        let seen_c = seen.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if matches!(call, HostCall::Begin { .. }) {
                *seen_c.borrow_mut() += 1;
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let out = interp.call("wf", vec![VoxValue::Int(5)]).expect("runs");
        assert!(matches!(out, VoxValue::Int(10)), "got {out:?}");
        assert_eq!(*seen.borrow(), 1, "aliased callee must still intercept");
    }

    #[test]
    fn replay_decision_skips_the_body() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount * 2 }
workflow wf(amount: int) to int { return charge(amount) }
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        interp.activity_hook = Some(Box::new(|call| {
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Replay(VoxValue::Int(100)),
                _ => HostDecision::Ack,
            })
        }));
        let out = interp.call("wf", vec![VoxValue::Int(5)]).expect("runs");
        assert!(matches!(out, VoxValue::Int(100)), "got {out:?}");
    }

    #[test]
    fn result_err_is_not_failed() {
        const SRC: &str = r#"
activity charge(amount: int) to Result[str] { return Error("declined") }
workflow wf(amount: int) to Result[str] { return charge(amount) }
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let failed_flag = Rc::new(RefCell::new(None));
        let failed_c = failed_flag.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::End { failed, .. } = &call {
                *failed_c.borrow_mut() = Some(*failed);
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let out = interp.call("wf", vec![VoxValue::Int(1)]).expect("runs");
        assert!(
            matches!(out, VoxValue::Result(Err(_))),
            "Err must flow; got {out:?}"
        );
        assert_eq!(
            *failed_flag.borrow(),
            Some(false),
            "Result::Err is not a crash"
        );
    }

    #[test]
    fn activity_panic_sends_failed_end_before_propagating() {
        const SRC: &str = r#"
activity charge() to int {
    assert(false)
    return 1
}
workflow wf() to int { return charge() }
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let calls = Rc::new(RefCell::new(Vec::<(&'static str, bool)>::new()));
        let calls_c = calls.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            match &call {
                HostCall::Begin { .. } => calls_c.borrow_mut().push(("Begin", false)),
                HostCall::End { failed, .. } => calls_c.borrow_mut().push(("End", *failed)),
                _ => {}
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));

        let err = interp.call("wf", vec![]).expect_err("panic must propagate");
        assert!(matches!(err, crate::eval::EvalError::AssertionFailed(_)));
        assert_eq!(*calls.borrow(), vec![("Begin", false), ("End", true)]);
    }

    #[test]
    fn with_options_reach_begin() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount }
workflow wf(amount: int) to int {
    return charge(amount) with { activity_id: "explicit-1" }
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let opts = Rc::new(RefCell::new(None));
        let opts_c = opts.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::Begin { options, .. } = &call {
                *opts_c.borrow_mut() = options.clone();
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let _ = interp
            .call("wf", vec![VoxValue::Int(1)])
            .expect("with must not PARITY_UNIMPLEMENTED");
        let some = opts.borrow();
        let some = some.as_ref().expect("Begin must carry with-options");
        match some {
            VoxValue::Object(fields) => {
                assert!(fields.iter().any(|(k, v)| k == "activity_id"
                    && matches!(v, VoxValue::Str(s) if s == "explicit-1")));
            }
            other => panic!("expected object options, got {other:?}"),
        }
    }

    #[test]
    fn workflow_version_is_a_patch_call() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount }
workflow wf(amount: int) to int {
    workflow.version("add-audit-v2", 1, 2)
    return charge(amount)
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let patches = Rc::new(RefCell::new(Vec::<String>::new()));
        let patches_c = patches.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::Patch { change_id, .. } = &call {
                patches_c.borrow_mut().push(change_id.clone());
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                HostCall::Patch { .. } => HostDecision::Ack,
                _ => HostDecision::Ack,
            })
        }));
        let _ = interp
            .call("wf", vec![VoxValue::Int(1)])
            .expect("version must not PARITY_UNIMPLEMENTED");
        assert_eq!(*patches.borrow(), vec!["add-audit-v2".to_string()]);
    }

    #[test]
    fn workflow_wait_is_a_wait_call() {
        const SRC: &str = r#"
activity charge(amount: int) to int { return amount }
workflow wf(amount: int) to int {
    workflow_wait(5000)
    return charge(amount)
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        interp.activity_hook = Some(Box::new(|call| {
            Ok(match call {
                HostCall::Wait { .. } => HostDecision::Park("timer".into()),
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let err = interp
            .call("wf", vec![VoxValue::Int(1)])
            .expect_err("wait must park");
        assert!(matches!(err, crate::eval::EvalError::WorkflowParked(ref r) if r == "timer"));
    }

    #[test]
    fn plain_functions_are_not_intercepted() {
        const SRC: &str = r#"
fn double(n: int) to int { return n * 2 }
activity charge(amount: int) to int { return amount }
workflow wf(amount: int) to int {
    let d = double(amount)
    return charge(d)
}
"#;
        let (mut interp, activities) = interp_for(SRC);
        interp.activity_names = activities;
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let seen_c = seen.clone();
        interp.activity_hook = Some(Box::new(move |call| {
            if let HostCall::Begin { name, .. } = &call {
                seen_c.borrow_mut().push(name.clone());
            }
            Ok(match call {
                HostCall::Begin { .. } => HostDecision::Execute,
                _ => HostDecision::Ack,
            })
        }));
        let _ = interp.call("wf", vec![VoxValue::Int(3)]).expect("runs");
        assert_eq!(*seen.borrow(), vec!["charge".to_string()]);
    }

    #[test]
    fn nested_workflow_calls_are_rejected() {
        const SRC: &str = r#"
workflow child(amount: int) to int { return amount }
workflow parent(amount: int) to int { return child(amount) }
"#;
        let hir = lower_module(&parse(lex(SRC)).expect("parses"));
        let mut interp = Interpreter::new(1_000_000);
        interp.workflow_names = hir
            .functions
            .iter()
            .filter(|f| f.durability == Some(crate::hir::nodes::DurabilityKind::Workflow))
            .map(|f| f.name.clone())
            .collect();
        interp.run_module(&hir).expect("module loads");

        let err = interp
            .call("parent", vec![VoxValue::Int(1)])
            .expect_err("nested workflow must fail");
        assert!(matches!(
            err,
            crate::eval::EvalError::Panic(ref message)
                if message == "nested workflow calls are not supported"
        ));
    }

    #[test]
    fn recursive_workflow_calls_are_rejected() {
        const SRC: &str = r#"
workflow wf() to int { return wf() }
"#;
        let hir = lower_module(&parse(lex(SRC)).expect("parses"));
        let mut interp = Interpreter::new(1_000_000);
        interp.workflow_names = ["wf".to_string()].into_iter().collect();
        interp.run_module(&hir).expect("module loads");

        let err = interp
            .call("wf", vec![])
            .expect_err("recursive workflow must fail");
        assert!(matches!(
            err,
            crate::eval::EvalError::Panic(ref message)
                if message == "nested workflow calls are not supported"
        ));
    }
}
