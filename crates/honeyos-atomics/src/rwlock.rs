use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError, TryLockResult};

/// A simple spin rwlock implementation.
/// Atomic waits panic on the main thread, so this is only safe to use in the user space.
pub trait SpinRwLock {
    type Inner;

    /// Spin until the read lock is available
    /// This will block the thread until the lock is available, so this cannot be called in the kernel.
    /// ### Errors
    /// - [`TryLockError::Poisoned`] When the lock is poisoned
    fn spin_read(&self) -> TryLockResult<RwLockReadGuard<Self::Inner>>;
    /// Spin until the write lock is available
    /// This will block the thread until the lock is available, so this cannot be called in the kernel.
    /// ### Errors
    /// - [`TryLockError::Poisoned`] When the lock is poisoned
    fn spin_write(&self) -> TryLockResult<RwLockWriteGuard<Self::Inner>>;
}

impl<T> SpinRwLock for RwLock<T> {
    type Inner = T;

    fn spin_read(&self) -> TryLockResult<RwLockReadGuard<Self::Inner>> {
        loop {
            match self.try_read() {
                Ok(guard) => return Ok(guard),
                Err(error) => match error {
                    TryLockError::WouldBlock => continue,
                    _ => return Err(error),
                },
            }
        }
    }

    fn spin_write(&self) -> TryLockResult<RwLockWriteGuard<Self::Inner>> {
        loop {
            match self.try_write() {
                Ok(guard) => return Ok(guard),
                Err(error) => match error {
                    TryLockError::WouldBlock => continue,
                    _ => return Err(error),
                },
            }
        }
    }
}
