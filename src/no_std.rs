extern crate core;

use self::core::cmp;
use self::core::iter;
use self::core::mem;
use self::core::ptr;
use self::core::sync::atomic;

use super::{CANARY_SIZE, CONF_ALLOC_EXTRA_MEM, get_mem_init};

extern crate spin;
use self::spin::Mutex;

extern crate typed_arena;
use self::typed_arena::Arena;

// NB: Every type's alignment is presumed valid when aligned to a multiple of usize's alignment.
const ARENA_SIZE_BYTES: usize = 8 * 1024 * 1024;
static ARENA: Mutex<Option<Arena<usize>>> = Mutex::new(None);

pub extern fn libdiffuzz_init_config() {
    // Initalize the arena with ugly pointer tricks since its constructor is not const_fn.
    let mut swap_arena = Some(Arena::with_capacity(ARENA_SIZE_BYTES / mem::size_of::<usize>()));
    unsafe {
        let guard_ref: &mut Option<Arena<usize>> = &mut ARENA.lock();
        ptr::swap(guard_ref, &mut swap_arena);
    }

    CONF_ALLOC_EXTRA_MEM.store(0, atomic::Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn malloc(len: usize) -> *mut u8 {
    let alloc_extra_mem = CONF_ALLOC_EXTRA_MEM.load(atomic::Ordering::Relaxed);
    let full_len = match len.checked_add(CANARY_SIZE).and_then(|x| x.checked_add(alloc_extra_mem)) {
        Some(x) => x,
        None => return ptr::null_mut(),
    };
    let usizes_len = full_len / mem::size_of::<usize>() +
        (full_len % mem::size_of::<usize>() > 0).into();
    let ptr: *mut usize = {
        let mut arena = ARENA.lock();
        let mem_init = unsafe {
            let mut bytes: usize = 0;
            ptr::write_bytes(&mut bytes, get_mem_init(), 1);
            bytes
        };
        let slice: &mut [usize] = arena.as_mut().unwrap().alloc_extend(iter::repeat(mem_init).take(usizes_len));
        slice.as_mut_ptr()
    };
    *ptr = full_len;
    (ptr as *mut u8).offset(CANARY_SIZE as isize)
}

#[no_mangle]
pub unsafe extern "C" fn calloc(n_items: usize, item_len: usize) -> *mut u8 {
    let len = match n_items.checked_mul(item_len) {
        Some(x) => x,
        None => return ptr::null_mut(),
    };
    malloc(len)
}

#[no_mangle]
pub unsafe extern "C" fn free(_ptr: *mut u8) {
    // Arenas don't free, so this is a no-op.
}

#[no_mangle]
pub unsafe extern "C" fn realloc(orig_ptr: *mut u8, new_len: usize) -> *mut u8 {
    if orig_ptr.is_null() {
        return malloc(new_len);
    }
    let orig_len = *(orig_ptr.offset(-(CANARY_SIZE as isize)) as *const usize);
    let new_ptr = malloc(new_len);
    ptr::copy_nonoverlapping(orig_ptr, new_ptr, cmp::min(new_len, orig_len));
    free(orig_ptr);
    new_ptr
}
