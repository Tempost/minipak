use crate::memmap::MmapOptions;
use linked_list_allocator::LockedHeap;

extern crate compiler_builtins;
extern crate rlibc;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE_MB: u64 = 128;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    crate::println!("{}", info);
    core::intrinsics::abort();
}

#[lang = "eh_personality"]
fn eh_personality() {}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
unsafe extern "C" fn _Unwind_Resume() {}

#[unsafe(no_mangle)]
unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe { compiler_builtins::mem::bcmp(s1, s2, n) }
}

pub unsafe fn init_allocator() {
    let heap_size = HEAP_SIZE_MB * 1024 * 1024;
    let heap_bottom = MmapOptions::new(heap_size).map().unwrap();
    unsafe {
        ALLOCATOR.lock().init(heap_bottom as _, heap_size as _);
    }
}
