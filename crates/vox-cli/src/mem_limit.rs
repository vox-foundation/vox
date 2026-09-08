//! Counting process allocator for `--max-memory`.
//!
//! Armed after CLI parse. Exceeding the ceiling writes a stack-formatted line
//! and terminates via `_exit` / `TerminateProcess` — never `std::process::exit`,
//! which runs atexit handlers that allocate and re-enter the allocator.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

#[repr(C, align(64))]
struct Line(AtomicUsize);

static USED: Line = Line(AtomicUsize::new(0));
static LIMIT: Line = Line(AtomicUsize::new(usize::MAX));

pub fn arm(bytes: usize) {
    USED.0.store(0, Ordering::Relaxed);
    LIMIT.0.store(bytes, Ordering::Relaxed);
}

struct Capped;
#[global_allocator]
static ALLOC: Capped = Capped;

unsafe impl GlobalAlloc for Capped {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        charge(layout.size());
        // SAFETY: `layout` is the caller's allocation request; forwarded to the system allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        charge(layout.size());
        // SAFETY: same contract as `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() {
            charge(new_size - layout.size());
        }
        // SAFETY: `ptr`/`layout` came from a prior allocation of this allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        uncharge(layout.size());
        // SAFETY: `ptr`/`layout` came from a prior allocation of this allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn charge(n: usize) {
    if LIMIT.0.load(Ordering::Relaxed) == usize::MAX {
        return;
    }
    let new = USED.0.fetch_add(n, Ordering::Relaxed) + n;
    if new > LIMIT.0.load(Ordering::Relaxed) {
        let _ = write_limit_line();
        die(79);
    }
}

fn uncharge(n: usize) {
    if LIMIT.0.load(Ordering::Relaxed) == usize::MAX {
        return;
    }
    USED.0.fetch_sub(n, Ordering::Relaxed);
}

fn write_limit_line() -> std::io::Result<()> {
    const MSG: &[u8] = b"vox: memory limit exceeded\n";
    #[cfg(unix)]
    {
        // SAFETY: `MSG` is a static byte string; fd 2 is stderr.
        let n = unsafe { libc::write(libc::STDERR_FILENO, MSG.as_ptr().cast(), MSG.len()) };
        if n < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[cfg(windows)]
    {
        unsafe {
            let handle = windows_sys::Win32::System::Console::GetStdHandle(
                windows_sys::Win32::System::Console::STD_ERROR_HANDLE,
            );
            let mut written = 0u32;
            let overlapped: *mut windows_sys::Win32::System::IO::OVERLAPPED = std::ptr::null_mut();
            let ok = windows_sys::Win32::Storage::FileSystem::WriteFile(
                handle,
                MSG.as_ptr(),
                MSG.len() as u32,
                &mut written,
                overlapped,
            );
            if ok == 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = MSG;
        Ok(())
    }
}

fn die(code: i32) -> ! {
    #[cfg(unix)]
    // SAFETY: `_exit` does not run atexit handlers (which would allocate).
    unsafe {
        libc::_exit(code);
    }
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Threading::TerminateProcess(
            windows_sys::Win32::System::Threading::GetCurrentProcess(),
            code as u32,
        );
        loop {
            std::hint::spin_loop();
        }
    }
    #[cfg(not(any(unix, windows)))]
    std::process::abort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_stores_the_ceiling() {
        arm(usize::MAX);
        assert_eq!(LIMIT.0.load(Ordering::Relaxed), usize::MAX);
    }

    #[test]
    fn charge_skips_when_limit_is_unset() {
        assert_eq!(LIMIT.0.load(Ordering::Relaxed), usize::MAX);
        let before = USED.0.load(Ordering::Relaxed);
        charge(1_000_000);
        assert_eq!(USED.0.load(Ordering::Relaxed), before);
    }

    #[test]
    fn uncharge_skips_when_unarmed() {
        assert_eq!(LIMIT.0.load(Ordering::Relaxed), usize::MAX);
        let before = USED.0.load(Ordering::Relaxed);
        uncharge(64);
        assert_eq!(USED.0.load(Ordering::Relaxed), before);
    }

    #[test]
    fn arm_resets_used() {
        USED.0.store(12345, Ordering::Relaxed);
        arm(usize::MAX);
        assert_eq!(USED.0.load(Ordering::Relaxed), 0);
        assert_eq!(LIMIT.0.load(Ordering::Relaxed), usize::MAX);
    }

    #[test]
    fn die_is_not_process_exit() {
        let src = include_str!("mem_limit.rs");
        let die_start = src.find("fn die(").expect("die() present");
        let after = &src[die_start..];
        let die_end = after.find("\n}").expect("die() body end");
        let die_fn = &after[..=die_end];
        assert!(
            !die_fn.contains("process::exit"),
            "die must not run atexit handlers"
        );
        #[cfg(unix)]
        assert!(die_fn.contains("_exit"));
        #[cfg(windows)]
        assert!(die_fn.contains("TerminateProcess"));
    }
}
