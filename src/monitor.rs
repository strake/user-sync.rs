use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// Mutual exclusionary primitive
pub struct Mutex<T: ?Sized> {
    lock: ::system::Mutex,
    valu: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    #[inline] pub const fn new(x: T) -> Self {
        Mutex {
            lock: ::system::Mutex::new(),
            valu: UnsafeCell::new(x),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Take an exclusive reference to the guarded value, blocking if another thread is already
    /// holding it.
    #[inline] pub fn lock(&self) -> Guard<T> {
        unsafe {
            self.lock.lock(0x100);
            Guard {
                lock: &self.lock,
                valu: &mut *self.valu.get(),
            }
        }
    }

    /// Take an exclusive reference to the guarded value, returning `None` if another thread is
    /// already holding it.
    #[inline] pub fn try_lock(&self) -> Option<Guard<T>> {
        unsafe {
            if self.lock.try_lock() {
                Some(Guard {
                    lock: &self.lock,
                    valu: &mut *self.valu.get(),
                })
            } else { None }
        }
    }
}

/// Exclusive reference to `Mutex`-guarded value
pub struct Guard<'a, T: ?Sized + 'a> {
    lock: &'a ::system::Mutex,
    valu: &'a mut T,
}

unsafe impl<'a, T: ?Sized> Send for Guard<'a, T> {}

impl<'a, T: ?Sized> Deref for Guard<'a, T> {
    type Target = T;
    #[inline] fn deref(&self) -> &T { self.valu }
}

impl<'a, T: ?Sized> DerefMut for Guard<'a, T> {
    #[inline] fn deref_mut(&mut self) -> &mut T { self.valu }
}

impl<'a, T: ?Sized> Drop for Guard<'a, T> {
    #[inline] fn drop(&mut self) { self.lock.unlock(0x100) }
}

/// Condition variable
///
/// A condition variable lets a thread holding a lock awaiting some predicate of the guarded
/// value to do so and not miss a notification that the predicate became true.
/// It may not be used with multiple mutexen simultaneously; this is dynamically asserted.
///
/// Example:
///
/// ```ignore
/// let mutex: Mutex<SomeQueueType<T>> = Mutex::new(_);
/// let cond = CondVar::new();
///
/// thread Consumer {
///     loop {
///         let mut guard = mutex.lock();
///         while guard.empty() {
///             // Wait for data to consume
///             n_guard = cond.wait(guard);
///         }
///         while *n_guard > 0 {
///             consume(guard.pop());
///         }
///     }
/// }
///
/// thread Producer {
///     loop {
///         let datum = produce();
///         mutex.lock().push(datum);
///         cond.notify_one();
///     }
/// }
/// ```
pub struct CondVar(::system::CondVar);

impl CondVar {
    #[inline] pub fn new() -> Self { CondVar(::system::CondVar::new()) }

    /// Atomically release the guard lock and wait for another thread to call `notify`.
    ///
    /// # Panics
    ///
    /// Panicks if the `CondVar` is already in use with another mutex.
    #[inline] pub fn wait<'a, T>(&self, guard: Guard<'a, T>) -> Guard<'a, T> {
        self.0.wait(guard.lock);
        guard
    }

    /// Unblock one waiting thread.
    #[inline] pub fn notify_one(&self) { self.0.notify_one() }

    /// Unblock all waiting threads.
    #[inline] pub fn notify_all(&self) { self.0.notify_all() }
}
