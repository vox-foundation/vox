//! Host capability probing and `requires-tag` satisfaction.
//!
//! `catalog.toml` entries carry an optional `requires-tag` (e.g. `"nvidia-gpu"`,
//! `"apple-silicon"`) naming a hardware/platform capability a plugin needs.
//! [`probe`] inspects the current host and produces the [`CapabilitySet`] of
//! tags it actually has; [`CapabilitySet::satisfies`] checks a plugin's
//! `requires-tag` against that set. Probing degrades gracefully: any failure
//! to detect a capability simply omits its tag, it never panics and never
//! returns an error.

use std::collections::BTreeSet;

/// Host hardware/platform tags a plugin's `requires-tag` is checked against.
/// Always includes `"cpu-only"`. Probing degrades to fewer tags on any
/// failure — it must never panic and never require a toolchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySet(BTreeSet<String>);

impl CapabilitySet {
    /// Build a set from an explicit list of tags (mainly for tests and
    /// callers that already know the host's capabilities).
    pub fn from_tags<I: IntoIterator<Item = T>, T: Into<String>>(tags: I) -> Self {
        Self(tags.into_iter().map(Into::into).collect())
    }

    /// `None` (no `requires-tag` declared) is always satisfied. `Some(tag)` is
    /// satisfied iff `tag` is in the set.
    pub fn satisfies(&self, requires_tag: Option<&str>) -> bool {
        match requires_tag {
            None => true,
            Some(tag) => self.0.contains(tag),
        }
    }
}

/// Probe this host's capabilities. Never panics; a probe failure for one
/// capability yields fewer tags, never a propagated error.
pub fn probe() -> CapabilitySet {
    let mut tags = BTreeSet::new();
    tags.insert("cpu-only".to_string());
    if cfg!(target_os = "macos") {
        // Every Mac vox can run on (macOS 11+, 2012 hardware onward) has a
        // Metal-capable GPU, Intel ones included. There is no lightweight
        // Metal-probe library in this tree (candle_core's Metal support needs
        // its heavy `metal` feature, which vox-plugin-host must not depend on
        // just to answer "is there a GPU"), so `metal` is derived from the
        // target OS rather than probed at runtime. The Metal plugin still
        // falls back to CPU with a warning if device creation fails.
        tags.insert("metal".to_string());
        if cfg!(target_arch = "aarch64") {
            tags.insert("apple-silicon".to_string());
        }
    }
    if cuda_driver_present() {
        tags.insert("nvidia-gpu".to_string());
    }
    // No `cuda-<major>` tag: getting one means resolving and calling
    // `cuDriverGetVersion`, which is a symbol lookup and a call into the
    // driver, not just a presence check — and the `SAFETY` argument for
    // `cuda_driver_present`'s `unsafe` block rests specifically on this
    // module never resolving or calling into an untrusted library. Nothing
    // in the catalog uses `requires-tag`s finer than `nvidia-gpu` today; add
    // the tag as its own deliberate, separately-reviewed follow-up if a
    // consumer ever needs a CUDA major-version distinction.
    CapabilitySet(tags)
}

/// CUDA driver library names for this target. The prefix and suffix come from
/// `libloading::library_filename`: `nvcuda.dll` on Windows, and `libcuda.so.1`
/// (the soname) then `libcuda.so` elsewhere.
fn cuda_driver_candidates() -> Vec<std::ffi::OsString> {
    if cfg!(target_os = "windows") {
        vec![libloading::library_filename("nvcuda")]
    } else if cfg!(target_os = "macos") {
        vec![] // CUDA ships no macOS driver on Apple Silicon or Intel since 2021.
    } else {
        let unversioned = libloading::library_filename("cuda");
        let mut soname = unversioned.clone();
        soname.push(".1");
        vec![soname, unversioned]
    }
}

/// Whether a CUDA-capable driver can be loaded on this host. Any load
/// failure (missing library, no permissions, wrong arch, unsupported OS)
/// means "not present", not an error to propagate.
// The workspace denies unsafe_code by default (Cargo.toml `[workspace.lints]`);
// this is the one deliberate FFI-adjacent exception for this task, see the
// SAFETY comment on the unsafe block below.
fn cuda_driver_present() -> bool {
    cuda_driver_candidates().iter().any(|name| {
        // SAFETY: `libloading::Library::new` is unsafe because loading a
        // shared library runs its initializers, but we only load
        // well-known system driver libraries by name to check whether they
        // exist and are loadable — no symbols are resolved or called here,
        // so there is no untrusted code path beyond what the OS's dynamic
        // linker already runs for any loaded library.
        #[allow(unsafe_code)]
        let loaded = unsafe { libloading::Library::new(name) };
        loaded.is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plugin_with_no_requires_tag_is_always_satisfied() {
        let caps = CapabilitySet::from_tags(["cpu-only"]);
        assert!(caps.satisfies(None));
    }

    #[test]
    fn a_requires_tag_must_be_present_in_the_probe() {
        let caps = CapabilitySet::from_tags(["apple-silicon", "metal"]);
        assert!(caps.satisfies(Some("apple-silicon")));
        assert!(!caps.satisfies(Some("nvidia-gpu")));
    }

    /// The derived names must equal the literals they replaced.
    #[test]
    fn cuda_driver_candidates_match_the_platform_driver_names() {
        let names: Vec<String> = cuda_driver_candidates()
            .into_iter()
            .map(|n| n.to_string_lossy().into_owned())
            .collect();
        #[cfg(target_os = "windows")]
        assert_eq!(names, ["nvcuda.dll"]);
        #[cfg(target_os = "macos")]
        assert!(names.is_empty());
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        assert_eq!(names, ["libcuda.so.1", "libcuda.so"]);
    }

    #[test]
    fn probe_never_panics_and_always_reports_cpu_only() {
        let caps = probe();
        assert!(caps.satisfies(Some("cpu-only")));
        // This test runs on macOS/aarch64 CI and dev machines: assert the probe
        // actually detected that, not merely that it returned *something*.
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        {
            assert!(caps.satisfies(Some("apple-silicon")));
            assert!(caps.satisfies(Some("metal")));
        }
        // No CUDA driver ships for macOS (post-2021, and never for Apple Silicon),
        // so this platform must never report nvidia-gpu.
        #[cfg(target_os = "macos")]
        assert!(!caps.satisfies(Some("nvidia-gpu")));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn every_mac_is_tagged_metal() {
        let caps = probe();
        assert!(
            caps.satisfies(Some("metal")),
            "all Macs since 2012 support Metal: {caps:?}"
        );
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    fn intel_mac_is_not_tagged_apple_silicon() {
        assert!(!probe().satisfies(Some("apple-silicon")));
    }
}
