//! Process priority helpers for `vox schola train`.
//!
//! When training while using the GPU for other work (browser, etc.), lowering
//! process priority keeps the system responsive.

/// Apply process priority. `low` = BELOW_NORMAL on Windows, nice 10 on Unix.
/// No-op when `normal` or when the OS call fails (log and continue).
#[cfg(feature = "gpu")]
#[allow(unsafe_code)] // Windows process APIs + `libc::setpriority`.
pub fn apply(priority: &str) {
    if priority != "low" {
        return;
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            BELOW_NORMAL_PRIORITY_CLASS, GetCurrentProcess, SetPriorityClass,
        };
        let handle = unsafe { GetCurrentProcess() };
        if handle.is_null() {
            tracing::warn!("process_priority: GetCurrentProcess failed");
            return;
        }
        if unsafe { SetPriorityClass(handle, BELOW_NORMAL_PRIORITY_CLASS) } == 0 {
            tracing::warn!("process_priority: SetPriorityClass failed");
        } else {
            tracing::debug!("process_priority: set to BELOW_NORMAL");
        }
    }
    #[cfg(unix)]
    {
        if unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, 10) } != 0 {
            tracing::warn!("process_priority: setpriority failed");
        } else {
            tracing::debug!("process_priority: set to nice 10");
        }
    }
}
