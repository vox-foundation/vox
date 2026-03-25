//! Recover from poisoned [`std::sync::RwLock`] / [`std::sync::Mutex`] guards.
//!
//! If a thread panics while holding a lock, subsequent `lock()` / `read()` / `write()`
//! return `PoisonError`. Unwrapping panics again and can brick long-lived services;
//! `PoisonError::into_inner` yields the guard so the map can keep working.

use std::sync::{RwLock as StdRwLock, RwLockReadGuard as StdRwLockReadGuard, RwLockWriteGuard as StdRwLockWriteGuard};

/// Read side of a [`StdRwLock`], recovering after poisoning.
pub fn rw_read<'a, T>(lock: &'a StdRwLock<T>) -> StdRwLockReadGuard<'a, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

/// Write side of a [`StdRwLock`], recovering after poisoning.
pub fn rw_write<'a, T>(lock: &'a StdRwLock<T>) -> StdRwLockWriteGuard<'a, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

