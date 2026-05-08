//! # vox-container
//!
//! OCI container runtime trait and types for the Vox toolchain.
//!
//! Provides [`ContainerRuntime`] trait + [`BuildOpts`] / [`RunOpts`].
//!
//! **Docker/Podman implementations** → `vox-plugin-runtime-container`
//! **Deployment artifact codegen** → `vox-deploy-codegen`
//! **Abstract skill runtime trait** → `vox-skill-runtime`
//! **Runtime detection** → `vox-plugin-runtime-container::detect_runtime`

#![allow(clippy::collapsible_if)]

mod runtime;

pub use runtime::{BuildOpts, ContainerRuntime, RunOpts};

/// Runtime preference enum — kept here for backward compat.
/// Callers should migrate to `vox_plugin_runtime_container::RuntimePreference`.
pub mod detect {
    /// Preferred container runtime selection strategy.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum RuntimePreference {
        /// Prefer Podman (rootless, daemonless), fall back to Docker.
        #[default]
        Auto,
        /// Use Docker only.
        Docker,
        /// Use Podman only.
        Podman,
    }

    impl std::str::FromStr for RuntimePreference {
        type Err = anyhow::Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.to_lowercase().as_str() {
                "auto" => Ok(Self::Auto),
                "docker" => Ok(Self::Docker),
                "podman" => Ok(Self::Podman),
                other => anyhow::bail!(
                    "Unknown runtime preference: {other:?}. Use auto, docker, or podman."
                ),
            }
        }
    }

    /// Detect and return the best available container runtime.
    ///
    /// Backward-compatibility shim — callers should migrate to
    /// `vox_plugin_runtime_container::detect_runtime`.
    ///
    /// # Errors
    /// Returns an error if neither Docker nor Podman is installed and reachable.
    pub fn detect_runtime(
        preference: RuntimePreference,
    ) -> anyhow::Result<Box<dyn crate::ContainerRuntime>> {
        use std::process::Command;

        fn docker_available() -> bool {
            Command::new("docker").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
        }
        fn podman_available() -> bool {
            Command::new("podman").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
        }

        // Inline struct impls to avoid circular dep with vox-plugin-runtime-container.
        struct DockerShim;
        struct PodmanShim;

        impl crate::ContainerRuntime for DockerShim {
            fn name(&self) -> &str { "docker" }
            fn available(&self) -> bool { docker_available() }
            fn version(&self) -> anyhow::Result<String> {
                let o = Command::new("docker").arg("--version").output()?;
                Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
            }
            fn build(&self, opts: &crate::BuildOpts) -> anyhow::Result<String> {
                let mut cmd = Command::new("docker");
                cmd.arg("build").arg("-t").arg(&opts.tag);
                if let Some(ref df) = opts.dockerfile { cmd.arg("-f").arg(df); }
                for (k, v) in &opts.build_args { cmd.arg("--build-arg").arg(format!("{k}={v}")); }
                cmd.arg(&opts.context_dir);
                let out = cmd.output()?;
                if !out.status.success() { anyhow::bail!("docker build failed:\n{}", String::from_utf8_lossy(&out.stderr)); }
                Ok(opts.tag.clone())
            }
            fn run(&self, opts: &crate::RunOpts) -> anyhow::Result<()> {
                let mut cmd = Command::new("docker");
                cmd.arg("run");
                if opts.detach { cmd.arg("-d"); }
                if opts.rm { cmd.arg("--rm"); }
                if let Some(ref n) = opts.name { cmd.arg("--name").arg(n); }
                for (h, c) in &opts.ports { cmd.arg("-p").arg(format!("{h}:{c}")); }
                for (k, v) in &opts.env { cmd.arg("-e").arg(format!("{k}={v}")); }
                for (h, c) in &opts.volumes { cmd.arg("-v").arg(format!("{h}:{c}")); }
                cmd.arg(&opts.image);
                crate::log_exec_risk(&opts.image);
                let st = cmd.status()?;
                if !st.success() { anyhow::bail!("docker run failed"); }
                Ok(())
            }
            fn push(&self, tag: &str) -> anyhow::Result<()> {
                let st = Command::new("docker").arg("push").arg(tag).status()?;
                if !st.success() { anyhow::bail!("docker push failed"); }
                Ok(())
            }
            fn tag(&self, src: &str, tgt: &str) -> anyhow::Result<()> {
                let st = Command::new("docker").arg("tag").arg(src).arg(tgt).status()?;
                if !st.success() { anyhow::bail!("docker tag failed"); }
                Ok(())
            }
            fn login(&self, registry: &str, username: &str, token: &str) -> anyhow::Result<()> {
                use std::io::Write;
                let mut child = Command::new("docker").arg("login").arg("-u").arg(username).arg("--password-stdin").arg(registry).stdin(std::process::Stdio::piped()).spawn()?;
                let mut stdin = child.stdin.take().unwrap();
                stdin.write_all(token.as_bytes())?;
                drop(stdin);
                let st = child.wait()?;
                if !st.success() { anyhow::bail!("docker login failed"); }
                Ok(())
            }
        }

        impl crate::ContainerRuntime for PodmanShim {
            fn name(&self) -> &str { "podman" }
            fn available(&self) -> bool { podman_available() }
            fn version(&self) -> anyhow::Result<String> {
                let o = Command::new("podman").arg("--version").output()?;
                Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
            }
            fn build(&self, opts: &crate::BuildOpts) -> anyhow::Result<String> {
                let mut cmd = Command::new("podman");
                cmd.arg("build").arg("--format").arg("oci").arg("-t").arg(&opts.tag);
                if let Some(ref df) = opts.dockerfile { cmd.arg("-f").arg(df); }
                for (k, v) in &opts.build_args { cmd.arg("--build-arg").arg(format!("{k}={v}")); }
                cmd.arg(&opts.context_dir);
                let out = cmd.output()?;
                if !out.status.success() { anyhow::bail!("podman build failed:\n{}", String::from_utf8_lossy(&out.stderr)); }
                Ok(opts.tag.clone())
            }
            fn run(&self, opts: &crate::RunOpts) -> anyhow::Result<()> {
                let mut cmd = Command::new("podman");
                cmd.arg("run");
                if opts.detach { cmd.arg("-d"); }
                if opts.rm { cmd.arg("--rm"); }
                if let Some(ref n) = opts.name { cmd.arg("--name").arg(n); }
                for (h, c) in &opts.ports { cmd.arg("-p").arg(format!("{h}:{c}")); }
                for (k, v) in &opts.env { cmd.arg("-e").arg(format!("{k}={v}")); }
                for (h, c) in &opts.volumes { cmd.arg("-v").arg(format!("{h}:{c}")); }
                cmd.arg(&opts.image);
                crate::log_exec_risk(&opts.image);
                let st = cmd.status()?;
                if !st.success() { anyhow::bail!("podman run failed"); }
                Ok(())
            }
            fn push(&self, tag: &str) -> anyhow::Result<()> {
                let st = Command::new("podman").arg("push").arg(tag).status()?;
                if !st.success() { anyhow::bail!("podman push failed"); }
                Ok(())
            }
            fn tag(&self, src: &str, tgt: &str) -> anyhow::Result<()> {
                let st = Command::new("podman").arg("tag").arg(src).arg(tgt).status()?;
                if !st.success() { anyhow::bail!("podman tag failed"); }
                Ok(())
            }
            fn login(&self, registry: &str, username: &str, token: &str) -> anyhow::Result<()> {
                use std::io::Write;
                let mut child = Command::new("podman").arg("login").arg("-u").arg(username).arg("--password-stdin").arg(registry).stdin(std::process::Stdio::piped()).spawn()?;
                let mut stdin = child.stdin.take().unwrap();
                stdin.write_all(token.as_bytes())?;
                drop(stdin);
                let st = child.wait()?;
                if !st.success() { anyhow::bail!("podman login failed"); }
                Ok(())
            }
        }

        match preference {
            RuntimePreference::Docker => {
                if docker_available() { Ok(Box::new(DockerShim)) }
                else { anyhow::bail!("Docker is not installed or not running.") }
            }
            RuntimePreference::Podman => {
                if podman_available() { Ok(Box::new(PodmanShim)) }
                else { anyhow::bail!("Podman is not installed.") }
            }
            RuntimePreference::Auto => {
                if podman_available() {
                    tracing::info!("Auto-detected Podman (rootless)");
                    Ok(Box::new(PodmanShim))
                } else if docker_available() {
                    tracing::info!("Auto-detected Docker");
                    Ok(Box::new(DockerShim))
                } else {
                    anyhow::bail!("No container runtime found (Docker or Podman).")
                }
            }
        }
    }
}

/// Classify the exec risk of a container image or command string and log the result.
pub fn log_exec_risk(raw_command: &str) {
    match vox_exec_grammar::parse(raw_command) {
        Ok(mut ast) => {
            let policy = vox_exec_grammar::ExecPolicy::default();
            vox_exec_grammar::risk::classify(&mut ast, &policy);
            tracing::info!(
                command = raw_command,
                risk = ?ast.risk,
                "exec-grammar risk classification"
            );
        }
        Err(e) => {
            tracing::debug!(
                command = raw_command,
                error = %e,
                "exec-grammar could not parse command; skipping risk classification"
            );
        }
    }
}
