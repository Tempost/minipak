#![allow(unsafe_op_in_unsafe_fn)]

use crate::memmap::MmapOptions;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE_MB: u64 = 128;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    crate::println!("{}", info);
    core::intrinsics::abort();
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
unsafe extern "C" fn _Unwind_Resume() {}

/// # Safety
/// Wild west of memory allocation
pub unsafe fn init_allocator() {
    let heap_size = HEAP_SIZE_MB * 1024 * 1024;
    let heap_bottom = MmapOptions::new(heap_size).map().unwrap();
    unsafe {
        ALLOCATOR.lock().init(heap_bottom as _, heap_size as _);
    }
}
