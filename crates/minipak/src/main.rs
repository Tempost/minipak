#![no_std]
#![no_main]
#![feature(naked_functions)]
#![allow(unsafe_op_in_unsafe_fn)]

use core::arch::naked_asm;
use encore::prelude::*;
mod cli;

#[allow(unused_attributes)]
unsafe extern "C" {}

#[naked]
#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    unsafe { naked_asm!("mov rdi, rsp", "call pre_main") }
}

#[unsafe(no_mangle)]
unsafe fn pre_main(stack_top: *mut u8) {
    unsafe {
        init_allocator();
    }
    main(Env::read(stack_top)).unwrap();
    syscall::exit(0);
}

fn main(env: Env) -> Result<(), EncoreError> {
    let args = cli::Args::parse(&env);

    let input = File::open(&args.input)?;
    let input = input.map()?;
    let input = input.as_ref();

    let compressed = lz4_flex::compress_prepend_size(input);
    let mut output = File::create(&args.output, 0o755)?;
    output.write_all(&compressed[..])?;

    println!(
        "Wrote {} ({:.2}% of input)",
        args.output,
        compressed.len() as f64 / input.len() as f64 * 100.0,
    );

    Ok(())
}
