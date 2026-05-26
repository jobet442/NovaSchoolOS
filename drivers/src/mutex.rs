#[cfg(not(target_os = "none"))]
pub struct Mutex<T> {
    inner: std::sync::Mutex<T>,
}

#[cfg(not(target_os = "none"))]
impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Mutex {
            inner: std::sync::Mutex::new(data),
        }
    }
    
    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard {
            inner: self.inner.lock().unwrap(),
        }
    }
}

#[cfg(not(target_os = "none"))]
pub struct MutexGuard<'a, T> {
    inner: std::sync::MutexGuard<'a, T>,
}

#[cfg(not(target_os = "none"))]
impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

#[cfg(not(target_os = "none"))]
impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "none")]
use core::cell::UnsafeCell;

#[cfg(target_os = "none")]
pub struct Mutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

#[cfg(target_os = "none")]
unsafe impl<T: Send> Sync for Mutex<T> {}
#[cfg(target_os = "none")]
unsafe impl<T: Send> Send for Mutex<T> {}

#[cfg(target_os = "none")]
impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Mutex {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        MutexGuard { mutex: self }
    }
}

#[cfg(target_os = "none")]
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

#[cfg(target_os = "none")]
impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

#[cfg(target_os = "none")]
impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

#[cfg(target_os = "none")]
impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.lock.store(false, Ordering::Release);
    }
}
