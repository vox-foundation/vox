//! Recover from poisoned [`std::sync::RwLock`] / [`std::sync::Mutex`] guards.
//!
//! If a thread panics while holding a lock, subsequent `lock()` / `read()` / `write()`
//! return `PoisonError`. Unwrapping panics again and can brick long-lived services;
//! `PoisonError::into_inner` yields the guard so the map can keep working.

use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Read side of a [`RwLock`], recovering after poisoning.
pub fn rw_read<'a, T>(lock: &'a RwLock<T>) -> RwLockReadGuard<'a, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

/// Write side of a [`RwLock`], recovering after poisoning.
pub fn rw_write<'a, T>(lock: &'a RwLock<T>) -> RwLockWriteGuard<'a, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}
