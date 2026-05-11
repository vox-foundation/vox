//! Mn-T4 / Mn-T5 acceptance: MENS decorators parse and participate in capability lowering.

use vox_compiler::ast::decl::Decl;
use vox_compiler::hir::HirCapability;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::typecheck_hir_module;

#[test]
fn inference_decorator_parses_model() {
    let src = r#"
        @inference(model = "llama")
        fn predict(x: str) to str {
            return x
        }
    "#;
    let m = parse(lex(src)).expect("parse");
    let Decl::Function(f) = &m.declarations[0] else {
        panic!("expected function");
    };
    assert_eq!(f.inference_model.as_deref(), Some("llama"));
}

#[test]
fn inference_lowers_gpu_random_net_caps() {
    let src = r#"
        @inference(model = "m")
        fn predict(x: str) to str {
            return x
        }
    "#;
    let m = parse(lex(src)).expect("parse");
    let hir = lower_module(&m);
    let f = hir
        .functions
        .iter()
        .find(|f| f.name == "predict")
        .expect("predict fn");
    assert!(f.capabilities.contains(&HirCapability::GpuCompute));
    assert!(f.capabilities.contains(&HirCapability::Random));
    assert!(f.capabilities.contains(&HirCapability::Net));
}

#[test]
fn training_step_lowers_caps() {
    let src = r#"
        @training_step
        fn step() to Unit {
            return Unit
        }
    "#;
    let m = parse(lex(src)).expect("parse");
    let hir = lower_module(&m);
    let f = hir.functions.iter().find(|f| f.name == "step").expect("step");
    assert!(f.training_step);
    assert!(f.capabilities.contains(&HirCapability::GpuCompute));
    assert!(f.capabilities.contains(&HirCapability::Mutate));
}

#[test]
#[serial_test::serial]
fn training_step_cuda_gate_emits_when_tier_low() {
    // SAFETY: serial_test ensures no concurrent env mutation in other tests.
    unsafe {
        std::env::set_var("VOX_CUDA_TIER", "0");
    }
    let src = r#"
        @training_step
        fn step() to Unit {
            return Unit
        }
    "#;
    let m = parse(lex(src)).expect("parse");
    let mut hir = lower_module(&m);
    let diags = typecheck_hir_module(src, &mut hir);
    unsafe {
        std::env::remove_var("VOX_CUDA_TIER");
    }
    assert!(
        diags.iter().any(|d| d.code.as_deref() == Some("vox/train/cuda-required")),
        "expected cuda-required diagnostic, got {diags:?}"
    );
}

#[test]
fn distributed_train_workflow_parses_metadata() {
    let src = r#"
        @distributed_train(strategy = "data_parallel", peers = 4)
        workflow Train() to Unit {
            return Unit
        }
    "#;
    let m = parse(lex(src)).expect("parse");
    let Decl::Workflow(w) = &m.declarations[0] else {
        panic!("expected workflow");
    };
    assert_eq!(
        w.distributed_train_strategy.as_deref(),
        Some("data_parallel")
    );
    assert_eq!(w.distributed_train_peers, Some(4));
}

#[test]
fn distributed_train_workflow_lowers_caps_and_meta() {
    let src = r#"
        @distributed_train(strategy = "data_parallel", peers = 4)
        workflow Train() to Unit {
            return Unit
        }
    "#;
    let m = parse(lex(src)).expect("parse");
    let hir = lower_module(&m);
    let f = hir.functions.iter().find(|f| f.name == "Train").expect("Train");
    assert_eq!(
        f.distributed_train.as_ref().map(|(s, p)| (s.as_str(), *p)),
        Some(("data_parallel", 4))
    );
    assert!(f.capabilities.contains(&HirCapability::Spawn));
    assert!(f.capabilities.contains(&HirCapability::Net));
}
