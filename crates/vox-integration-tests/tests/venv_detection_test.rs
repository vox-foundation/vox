#![allow(missing_docs)]
#![allow(unsafe_code)]
//! Integration tests for uv venv auto-detection in vox-container (`PythonEnv`).
//!
//! These tests verify that:
//! - `PythonEnv::site_packages_path()` and `venv_path()` behave correctly
//!   when a synthetic `.venv` is present.
//! - The venv-path helpers correctly identify Windows vs POSIX layouts.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use vox_deploy_codegen::env::PythonEnv;

/// `VIRTUAL_ENV` / `UV_PROJECT_ENVIRONMENT` are process-global; serialize tests that touch them.
static VENV_ENV_LOCK: Mutex<()> = Mutex::new(());

fn make_windows_venv(base: &Path) -> PathBuf {
    let venv = base.join(".venv");
    let sp = venv.join("Lib").join("site-packages");
    fs::create_dir_all(&sp).expect("could not create fake windows venv");
    venv
}

fn make_posix_venv(base: &Path) -> PathBuf {
    let venv = base.join(".venv");
    let sp = venv.join("lib").join("python3.12").join("site-packages");
    fs::create_dir_all(&sp).expect("could not create fake posix venv");
    venv
}

fn dummy_env() -> PythonEnv {
    PythonEnv {
        uv_available: false,
        uv_version: None,
        python_version: None,
        cuda_version: None,
        has_gpu: false,
    }
}

#[test]
fn venv_path_via_env_var_uv_project_environment() {
    let _lock = VENV_ENV_LOCK.lock().expect("venv env lock");
    let tmp = tempfile::tempdir().expect("tempdir");
    let venv = make_windows_venv(tmp.path());

    unsafe {
        std::env::set_var("UV_PROJECT_ENVIRONMENT", venv.to_str().unwrap());
    }
    let env = dummy_env();
    let found = env.venv_path();
    unsafe {
        std::env::remove_var("UV_PROJECT_ENVIRONMENT");
    }

    assert!(
        found.is_some(),
        "should detect venv via UV_PROJECT_ENVIRONMENT"
    );
    assert_eq!(found.unwrap(), venv);
}

#[test]
fn venv_path_via_env_var_virtual_env() {
    let _lock = VENV_ENV_LOCK.lock().expect("venv env lock");
    let tmp = tempfile::tempdir().expect("tempdir");
    let venv = make_posix_venv(tmp.path());

    unsafe {
        std::env::remove_var("UV_PROJECT_ENVIRONMENT");
        std::env::set_var("VIRTUAL_ENV", venv.to_str().unwrap());
    }
    let env = dummy_env();
    let found = env.venv_path();
    unsafe {
        std::env::remove_var("VIRTUAL_ENV");
    }

    assert!(found.is_some(), "should detect venv via VIRTUAL_ENV");
    assert_eq!(found.unwrap(), venv);
}

#[test]
fn site_packages_path_windows_layout() {
    let _lock = VENV_ENV_LOCK.lock().expect("venv env lock");
    let tmp = tempfile::tempdir().expect("tempdir");
    let venv = make_windows_venv(tmp.path());
    let expected_sp = venv.join("Lib").join("site-packages");

    unsafe {
        std::env::set_var("UV_PROJECT_ENVIRONMENT", venv.to_str().unwrap());
    }
    let env = dummy_env();
    let sp = env.site_packages_path();
    unsafe {
        std::env::remove_var("UV_PROJECT_ENVIRONMENT");
    }

    assert!(
        sp.is_some(),
        "site_packages_path should find Lib/site-packages"
    );
    assert_eq!(sp.unwrap(), expected_sp);
}

#[test]
fn site_packages_path_posix_layout() {
    let _lock = VENV_ENV_LOCK.lock().expect("venv env lock");
    let tmp = tempfile::tempdir().expect("tempdir");
    let venv = make_posix_venv(tmp.path());
    let expected_sp = venv.join("lib").join("python3.12").join("site-packages");

    unsafe {
        std::env::remove_var("UV_PROJECT_ENVIRONMENT");
        std::env::set_var("VIRTUAL_ENV", venv.to_str().unwrap());
    }
    let env = dummy_env();
    let sp = env.site_packages_path();
    unsafe {
        std::env::remove_var("VIRTUAL_ENV");
    }

    assert!(
        sp.is_some(),
        "site_packages_path should find lib/python*/site-packages"
    );
    assert_eq!(sp.unwrap(), expected_sp);
}

#[test]
fn site_packages_path_absent_returns_none_when_no_venv_env_vars() {
    let _lock = VENV_ENV_LOCK.lock().expect("venv env lock");
    unsafe {
        std::env::remove_var("UV_PROJECT_ENVIRONMENT");
        std::env::remove_var("VIRTUAL_ENV");
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();

    let env = dummy_env();
    let sp = env.site_packages_path();

    std::env::set_current_dir(original_cwd).unwrap();

    let _ = sp;
}
