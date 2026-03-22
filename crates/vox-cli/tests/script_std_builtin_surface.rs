//! Alignment: `std.*` script builtins across typeck, codegen, and runtime.

#[test]
fn glob_and_run_capture_exist_in_all_layers() {
    let checker = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-typeck/src/checker.rs"
    ));
    let emit = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-codegen-rust/src/emit.rs"
    ));
    let builtins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-runtime/src/builtins.rs"
    ));

    assert!(checker.contains("\"glob\"") && checker.contains("StdFsNs"));
    assert!(emit.contains("(\"fs\", \"glob\")"));
    assert!(builtins.contains("fn vox_fs_glob"));

    assert!(checker.contains("run_capture") && checker.contains("StdProcessNs"));
    assert!(emit.contains("run_capture"));
    assert!(builtins.contains("fn vox_process_run_capture"));

    assert!(checker.contains("run_capture_ex") && checker.contains("StdJsonNs"));
    assert!(emit.contains("(\"json\", \"read_str\")"));
    assert!(emit.contains("join_many"));
    assert!(builtins.contains("fn vox_json_read_str"));
}

#[test]
fn std_env_fs_path_process_core_methods_align() {
    let checker = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-typeck/src/checker.rs"
    ));
    let emit = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-codegen-rust/src/emit.rs"
    ));
    let builtins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-runtime/src/builtins.rs"
    ));

    assert!(checker.contains("StdEnvNs") && checker.contains("\"get\""));
    assert!(emit.contains("(\"env\", \"get\")"));
    assert!(builtins.contains("fn vox_env_get"));

    assert!(checker.contains("StdFsNs") && checker.contains("\"list_dir\""));
    assert!(emit.contains("(\"fs\", \"list_dir\")"));
    assert!(builtins.contains("fn vox_list_dir"));

    assert!(checker.contains("StdPathNs") && checker.contains("\"join\""));
    assert!(emit.contains("(\"path\", \"join\")"));

    assert!(checker.contains("StdProcessNs") && checker.contains("\"run\""));
    assert!(emit.contains("(\"process\", \"run\")"));
    assert!(builtins.contains("fn vox_process_run"));

    assert!(checker.contains("remove_dir_all"));
    assert!(emit.contains("(\"fs\", \"remove_dir_all\")"));
    assert!(builtins.contains("fn vox_fs_remove_dir_all"));
}
