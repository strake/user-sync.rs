/// Thread barrier
#[derive(Debug)]
pub struct Barrier(::system::Barrier);

impl Barrier {
    /// Make a barrier for `n` threads.
    #[inline] pub const fn new(n: usize) -> Self { Barrier(::system::Barrier::new(n)) }

    /// Wait until all threads reach the barrier.
    /// Returns `true` in only one arbitrary thread and `false` in the rest.
    #[inline] pub fn wait(&self) -> bool { self.0.wait() }
}
