#![feature(const_fn)]
#![feature(integer_atomics)]

#![no_std]

#[macro_use] extern crate syscall;

#[cfg(target_os = "linux")] mod linux;
#[cfg(target_os = "linux")] use ::linux as system;

mod barrier;
mod monitor;

pub use barrier::*;
pub use monitor::*;
