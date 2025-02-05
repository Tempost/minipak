#![no_std]
#![feature(naked_functions)]
#![allow(unsafe_op_in_unsafe_fn)]
extern crate alloc;
use core::arch::naked_asm;

use encore::prelude::*;
macro_rules! info {
    ($($tokens: tt)*) => {
        println!("[stage1] {}", alloc::format!($($tokens)*));
    }
}

/// # Safety
/// Using inline assembly so as to behave
/// as the entry point of a static exe
#[unsafe(no_mangle)]
#[naked]
pub unsafe extern "C" fn entry() {
    naked_asm!("mov rdi, rsp", "call premain");
}

/// # Safety
/// Init allocator
#[unsafe(no_mangle)]
#[inline(never)]
unsafe fn premain(stack_top: *mut u8) -> ! {
    init_allocator();
    crate::main(stack_top)
}

#[inline(never)]
unsafe fn main(stack_top: *mut u8) -> ! {
    info!("Stack top: {:?}", stack_top);
    syscall::exit(0)
}
