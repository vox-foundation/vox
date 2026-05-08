//! Alignment: `std.*` script builtins across typeck, codegen (`stmt_expr` → `builtin_registry`), and runtime.

#[test]
fn glob_and_run_capture_exist_in_all_layers() {
    let checker = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-compiler/src/typeck/checker/expr_field.rs"
    ));
    let registry = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-compiler/src/builtin_registry.rs"
    ));
    let emit = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-compiler-emit/src/codegen_rust/emit/stmt_expr.rs"
    ));
    let builtins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-runtime/src/builtins/mod.rs"
    ));

    assert!(emit.contains("std_namespace_runtime_call"));
    assert!(registry.contains("(\"fs\", \"glob\")") && checker.contains("StdFsNs"));
    assert!(registry.contains("vox_fs_glob"));
    assert!(builtins.contains("fn vox_fs_glob"));

    assert!(
        registry.contains("(\"process\", \"run_capture\")") && checker.contains("StdProcessNs")
    );
    assert!(registry.contains("vox_process_run_capture"));
    assert!(builtins.contains("fn vox_process_run_capture"));

    assert!(
        registry.contains("(\"process\", \"run_capture_ex\")") && checker.contains("StdProcessNs")
    );
    assert!(registry.contains("(\"json\", \"read_str\")") && checker.contains("StdJsonNs"));
    assert!(registry.contains("vox_json_read_str"));
    assert!(registry.contains("(\"path\", \"join_many\")"));
    assert!(builtins.contains("fn vox_json_read_str"));
}

#[test]
fn std_env_fs_path_process_core_methods_align() {
    let checker = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-compiler/src/typeck/checker/expr_field.rs"
    ));
    let registry = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-compiler/src/builtin_registry.rs"
    ));
    let emit = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-compiler-emit/src/codegen_rust/emit/stmt_expr.rs"
    ));
    let builtins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-runtime/src/builtins/mod.rs"
    ));

    assert!(emit.contains("std_namespace_runtime_call"));
    assert!(checker.contains("StdEnvNs") && registry.contains("(\"env\", \"get\")"));
    assert!(registry.contains("vox_env_get"));
    assert!(builtins.contains("fn vox_env_get"));

    assert!(checker.contains("StdFsNs") && registry.contains("(\"fs\", \"list_dir\")"));
    assert!(builtins.contains("fn vox_list_dir"));

    assert!(checker.contains("StdPathNs") && registry.contains("(\"path\", \"join\")"));

    assert!(checker.contains("StdProcessNs") && registry.contains("(\"process\", \"run\")"));
    assert!(builtins.contains("fn vox_process_run"));

    assert!(
        registry.contains("(\"process\", \"which\")") && registry.contains("vox_process_which")
    );
    assert!(builtins.contains("fn vox_process_which"));

    assert!(checker.contains("StdFsNs") && registry.contains("(\"fs\", \"remove_dir_all\")"));
    assert!(builtins.contains("fn vox_fs_remove_dir_all"));
}
