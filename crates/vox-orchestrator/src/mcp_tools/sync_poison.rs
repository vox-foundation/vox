//! RwLock poison handling for MCP tool surfaces.

use std::sync::{LockResult, RwLockReadGuard as StdRead, RwLockWriteGuard as StdWrite};

pub trait PoisonHandler<T: ?Sized> {
    type Guard;
    fn handle(self, context: &'static str) -> anyhow::Result<Self::Guard>;
}

impl<'a, T: ?Sized> PoisonHandler<T> for LockResult<StdRead<'a, T>> {
    type Guard = StdRead<'a, T>;
    fn handle(self, context: &'static str) -> anyhow::Result<StdRead<'a, T>> {
        self.map_err(|e| anyhow::anyhow!("{context}: {e}"))
    }
}

impl<'a, T: ?Sized> PoisonHandler<T> for LockResult<StdWrite<'a, T>> {
    type Guard = StdWrite<'a, T>;
    fn handle(self, context: &'static str) -> anyhow::Result<StdWrite<'a, T>> {
        self.map_err(|e| anyhow::anyhow!("{context}: {e}"))
    }
}

impl<'a, T: ?Sized> PoisonHandler<T> for parking_lot::RwLockReadGuard<'a, T> {
    type Guard = parking_lot::RwLockReadGuard<'a, T>;
    fn handle(self, _context: &'static str) -> anyhow::Result<Self::Guard> {
        Ok(self)
    }
}

impl<'a, T: ?Sized> PoisonHandler<T> for parking_lot::RwLockWriteGuard<'a, T> {
    type Guard = parking_lot::RwLockWriteGuard<'a, T>;
    fn handle(self, _context: &'static str) -> anyhow::Result<Self::Guard> {
        Ok(self)
    }
}

pub fn poison_rw_read<T: ?Sized, P: PoisonHandler<T>>(
    res: P,
    context: &'static str,
) -> anyhow::Result<P::Guard> {
    res.handle(context)
}

pub fn poison_rw_write<T: ?Sized, P: PoisonHandler<T>>(
    res: P,
    context: &'static str,
) -> anyhow::Result<P::Guard> {
    res.handle(context)
}
