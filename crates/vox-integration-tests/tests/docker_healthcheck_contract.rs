//! D04: Repo Dockerfiles must use a stable OCI health probe (no grep on human `vox doctor` text).

use std::fs;
use std::path::PathBuf;

fn root_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name)
}

#[test]
fn root_dockerfile_uses_doctor_probe() {
    let df = fs::read_to_string(root_file("Dockerfile")).expect("read Dockerfile");
    assert!(
        df.contains("doctor --probe"),
        "root Dockerfile HEALTHCHECK should invoke `vox doctor --probe`"
    );
    assert!(
        !df.contains("grep -Eq"),
        "root Dockerfile should not rely on grep against doctor stdout"
    );
}

#[test]
fn populi_dockerfile_uses_doctor_probe() {
    let df =
        fs::read_to_string(root_file("docker/Dockerfile.populi")).expect("read populi Dockerfile");
    assert!(
        df.contains("doctor --probe"),
        "docker/Dockerfile.populi HEALTHCHECK should invoke `vox doctor --probe`"
    );
    assert!(
        !df.contains("grep -Eq"),
        "populi Dockerfile should not rely on grep against doctor stdout"
    );
}
