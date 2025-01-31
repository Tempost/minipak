#![no_std]
#![no_main]
#![feature(naked_functions)]

use core::arch::naked_asm;
use encore::prelude::*;

#[allow(unused_attributes)]
unsafe extern "C" {}

#[naked]
#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    unsafe { naked_asm!("mov rdi, rsp", "call pre_main") }
}

#[unsafe(no_mangle)]
unsafe fn pre_main(_stack_top: *mut u8) {
    unsafe {
        init_allocator();
    }
    main().unwrap();
    syscall::exit(0);
}

fn main() -> Result<(), EncoreError> {
    let file = File::open("/lib64/ld-linux-x86-64.so.2")?;
    let map = file.map()?;

    let s = core::str::from_utf8(&map[1..4]).unwrap();
    println!("{}", s);

    Ok(())
}
