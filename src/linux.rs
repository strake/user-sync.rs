use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering as Memord,
                         spin_loop_hint as cpu_relax};

// 0: unlocked
// 1: locked
// 2: locked and contended
pub struct Mutex(AtomicU32);

impl Mutex {
    #[inline] pub const fn new() -> Self { Mutex(AtomicU32::new(0)) }

    #[inline] pub fn lock(&self, spins: usize) {
        let &Mutex(ref m) = self;
        let mut n = 0;
        for _ in 0..spins {
            n = m.compare_and_swap(0, 1, Memord::Acquire);
            if n == 0 { return };
            cpu_relax();
        }
        if n == 1 { n = m.swap(2, Memord::AcqRel) };
        while n > 0 {
            futex_wait(&m, 2);
            n = m.swap(2, Memord::Release);
        }
    }

    #[inline] pub fn unlock(&self, spins: usize) {
        let &Mutex(ref m) = self;
        if m.swap(0, Memord::Release) == 1 { return };
        for _ in 0..spins {
            if m.load(Memord::Acquire) > 0 &&
               m.compare_and_swap(1, 2, Memord::AcqRel) > 0 { return };
            cpu_relax();
        }
        futex_wake(&m, 1);
    }

    #[inline] pub fn try_lock(&self) -> bool {
        self.0.compare_and_swap(0, 1, Memord::Acquire) == 0
    }
}

pub struct Barrier {
    n_waiting: AtomicUsize,
    n_total: usize,
    seq: AtomicU32,
}

impl Barrier {
    #[inline] pub const fn new(n: usize) -> Self {
        Barrier {
            n_waiting: AtomicUsize::new(0),
            n_total: n,
            seq: AtomicU32::new(0 /* mem::uninitialized */),
        }
    }

    #[inline] pub fn wait(&self) -> bool {
        loop {
            let seq = self.seq.load(Memord::Acquire);
            let n_waiting = self.n_waiting.fetch_add(1, Memord::Relaxed) + 1;
            if n_waiting < self.n_total {
                while self.seq.load(Memord::Relaxed) == seq {
                    futex_wait(&self.seq, seq);
                }
                return false;
            }
            if n_waiting == self.n_total {
                self.n_waiting.store(0, Memord::Relaxed);
                self.seq.fetch_add(1, Memord::Release);
                futex_wake(&self.seq, !0);
                return true;
            }
            panic!("Too many waiters");
        }
    }
}

pub struct CondVar {
    ptr: AtomicPtr<AtomicU32>,
    seq: AtomicU32,
}

impl CondVar {
    #[inline] pub const fn new() -> Self {
        CondVar {
            ptr: AtomicPtr::new(ptr::null_mut()),
            seq: AtomicU32::new(0 /* mem::uninitialized */),
        }
    }

    #[inline] pub fn wait(&self, m: &Mutex) {
        debug_assert_ne!(0, m.0.load(Memord::Relaxed));

        let seq = self.seq.load(Memord::Acquire);
        let ptr = self.ptr.load(Memord::Acquire);
        if ptr.is_null() {
            self.ptr.compare_and_swap(ptr::null_mut(), ptr, Memord::AcqRel);
            debug_assert_eq!(ptr, self.ptr.load(Memord::Relaxed),
                             "CondVar used with multiple Mutexen");
        } else {
            debug_assert_eq!(ptr as *const _, &m.0, "CondVar used with multiple Mutexen");
        }

        m.unlock(0x100);

        futex_wait(&self.seq, seq);

        while 0 != m.0.swap(2, Memord::AcqRel) {
            futex_wait(&m.0, 2);
        }
    }

    #[inline] pub fn notify_one(&self) {
        self.seq.fetch_add(1, Memord::Relaxed);
        futex_wake(&self.seq, 1);
    }

    #[inline] pub fn notify_all(&self) {
        let ptr = self.ptr.load(Memord::Acquire);
        if ptr.is_null() { return; }
        self.seq.fetch_add(1, Memord::Relaxed);
        futex_reque(&self.seq, 1, !0, ptr);
    }
}

const FUTEX_WAIT: usize = 0;
const FUTEX_WAKE: usize = 1;
const FUTEX_PRIVATE_FLAG: usize = 0x80;

#[inline]
pub fn futex_wait(f: &AtomicU32, val: u32) { unsafe {
    syscall!(FUTEX, f as *const _, FUTEX_WAIT | FUTEX_PRIVATE_FLAG, val, 0);
} }

#[inline]
pub fn futex_wake(f: &AtomicU32, n: usize) -> usize { unsafe {
    syscall!(FUTEX, f as *const _, FUTEX_WAKE | FUTEX_PRIVATE_FLAG, n)
} }

#[inline]
pub fn futex_reque(f: &AtomicU32, n: usize, m: usize, ptr: *mut AtomicU32) -> usize { unsafe {
    syscall!(FUTEX, f as *const _, n, m, ptr)
} }
