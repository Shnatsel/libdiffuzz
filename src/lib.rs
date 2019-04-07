#![cfg_attr(not(feature = "have_std"), no_std)]

#[cfg(not(feature = "have_std"))]
use core as std;

use std::mem;
use std::sync::atomic::{self, AtomicUsize};

const CANARY_SIZE: usize = mem::size_of::<usize>();

static MEM_INIT: AtomicUsize = atomic::ATOMIC_USIZE_INIT;
static CONF_ALLOC_EXTRA_MEM: AtomicUsize = atomic::ATOMIC_USIZE_INIT;

#[cfg(feature = "have_std")]
mod have_std;
#[cfg(feature = "have_std")]
use have_std::libdiffuzz_init_config;

#[cfg(not(feature = "have_std"))]
mod no_std;
#[cfg(not(feature = "have_std"))]
use no_std::libdiffuzz_init_config;

#[cfg_attr(any(target_os = "macos", target_os = "ios"), link_section = "__DATA,__mod_init_func")]
#[cfg_attr(not(any(target_os = "macos", target_os = "ios")), link_section = ".ctors")]
pub static CONSTRUCTOR: extern fn() = libdiffuzz_init_config;

/// Gets then increments MEM_INIT
fn get_mem_init() -> u8 {
    MEM_INIT.fetch_add(1, atomic::Ordering::Relaxed) as u8
}
