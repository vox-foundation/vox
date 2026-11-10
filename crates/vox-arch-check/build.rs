use std::process::Command;

fn main() {
    emit();
    println!("cargo:rerun-if-changed=build.rs");
}

fn emit() {
    println!("cargo:rustc-env=VOX_BUILD_NUMBER={}", build_number());

    let git_hash =
        git_stdout(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=VOX_GIT_HASH={git_hash}");

    // Invalidate when the branch tip changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");
}

fn build_number() -> String {
    if let Ok(explicit) = std::env::var("VOX_BUILD_NUMBER")
        && !explicit.trim().is_empty()
    {
        return explicit.trim().to_string();
    }
    if git_stdout(&["rev-parse", "--is-shallow-repository"]).as_deref() == Some("true") {
        return "dev".to_string();
    }
    git_stdout(&["rev-list", "--count", "HEAD"]).unwrap_or_else(|| "dev".to_string())
}

fn git_stdout(args: &[&str]) -> Option<String> {
    // vox-arch-check: allow git-exec
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
