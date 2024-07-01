use std::sync::{MutexGuard, TryLockResult};

/// A simple spin mutex implementation.
/// Atomic waits panic on the main thread, so this is only safe to use in the user space.
pub trait SpinMutex {
    type Inner;

    /// Spin until the read lock is available
    /// This will block the thread until the lock is available, so this cannot be called in the kernel.
    /// ### Errors
    /// - [`TryLockError::Poisoned`] When the lock is poisoned
    fn spin_lock(&self) -> TryLockResult<MutexGuard<Self::Inner>>;
}

impl<T> SpinMutex for std::sync::Mutex<T> {
    type Inner = T;

    fn spin_lock(&self) -> TryLockResult<MutexGuard<Self::Inner>> {
        loop {
            // log::info!("Spin Lock started for: {}", std::any::type_name::<T>());
            std::thread::sleep(std::time::Duration::from_millis(50)); // Will panic on main thread
            match self.try_lock() {
                Ok(guard) => {
                    // log::info!("Spin Lock stopped for: {}", std::any::type_name::<T>());
                    return Ok(guard);
                }
                Err(error) => match error {
                    std::sync::TryLockError::WouldBlock => continue,
                    _ => return Err(error),
                },
            }
        }
    }
}
