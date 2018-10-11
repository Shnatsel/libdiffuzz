extern crate libc;
extern crate rand;

use libc::c_void;
use rand::Rng;
use std::cmp;
use std::env;
use std::mem;
use std::ptr;
use std::sync::atomic::{self, AtomicUsize};

const CANARY_SIZE: usize = mem::size_of::<usize>();

static MEM_INIT: AtomicUsize = atomic::ATOMIC_USIZE_INIT;
static CONF_ALLOC_EXTRA_MEM: AtomicUsize = atomic::ATOMIC_USIZE_INIT;

pub extern fn libdiffuzz_init_config() {
    if env::var_os("LIBDIFFUZZ_NONDETERMINISTIC").is_some() {
        MEM_INIT.store(
            rand::thread_rng().gen::<u8>().into(),
            atomic::Ordering::Relaxed,
        );
    }
    let alloc_extra_mem = env::var("LIBDIFFUZZ_ALLOCATE_EXTRA_MEMORY")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(0);
    CONF_ALLOC_EXTRA_MEM.store(alloc_extra_mem, atomic::Ordering::Relaxed);
}

#[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
#[cfg_attr(not(target_os = "macos"), link_section = ".ctors")]
pub static CONSTRUCTOR: extern fn() = libdiffuzz_init_config;

/// Gets then increments MEM_INIT
fn get_mem_init() -> u8 {
    MEM_INIT.fetch_add(1, atomic::Ordering::Relaxed) as u8
}

#[no_mangle]
pub unsafe extern "C" fn malloc(len: usize) -> *mut c_void {
    let alloc_extra_mem = CONF_ALLOC_EXTRA_MEM.load(atomic::Ordering::Relaxed);
    let full_len = match len.checked_add(CANARY_SIZE).and_then(|x| x.checked_add(alloc_extra_mem)) {
        Some(x) => x,
        None => return ptr::null_mut(),
    };
    let mut ptr = libc::mmap(
        ptr::null_mut(),
        full_len,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
        -1,
        0,
    );
    // This is guaranteed to be aligned
    *(ptr as *mut usize) = full_len;
    ptr = ptr.offset(CANARY_SIZE as isize);
    libc::memset(ptr, get_mem_init().into(), len + alloc_extra_mem);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn calloc(n_items: usize, item_len: usize) -> *mut c_void {
    let len = match n_items.checked_mul(item_len) {
        Some(x) => x,
        None => return ptr::null_mut(),
    };
    let alloc_extra_mem = CONF_ALLOC_EXTRA_MEM.load(atomic::Ordering::Relaxed);
    let full_len = match len.checked_add(CANARY_SIZE).and_then(|x| x.checked_add(alloc_extra_mem)) {
        Some(x) => x,
        None => return ptr::null_mut(),
    };
    let mut ptr = libc::mmap(
        ptr::null_mut(),
        full_len,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
        -1,
        0,
    );
    // This is guaranteed to be aligned
    *(ptr as *mut usize) = full_len;
    ptr = ptr.offset(CANARY_SIZE as isize);
    libc::memset(ptr, 0, len);
    libc::memset(
        ptr.offset(len as isize),
        get_mem_init().into(),
        alloc_extra_mem,
    );
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let real_ptr = ptr.offset(-(CANARY_SIZE as isize));
    libc::munmap(real_ptr, *(real_ptr as *const usize));
}

#[no_mangle]
pub unsafe extern "C" fn realloc(orig_ptr: *mut c_void, new_len: usize) -> *mut c_void {
    if orig_ptr.is_null() {
        return malloc(new_len);
    }
    let orig_len = *(orig_ptr.offset(-(CANARY_SIZE as isize)) as *const usize);
    let new_ptr = malloc(new_len);
    libc::memcpy(new_ptr, orig_ptr, cmp::min(new_len, orig_len));
    free(orig_ptr);
    new_ptr
}
