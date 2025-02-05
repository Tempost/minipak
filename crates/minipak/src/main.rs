#![no_std]
#![no_main]
#![feature(naked_functions)]
#![allow(unsafe_op_in_unsafe_fn)]

mod cli;
mod error;

use core::{arch::naked_asm, ops::Range};
use encore::prelude::*;
use error::Error;
use pixie::{MappedObject, Object, Writer};

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

fn main(env: Env) -> Result<(), Error> {
    let args = cli::Args::parse(&env);
    println!("Packing guest {:?}", args.input);
    let guest_file = File::open(args.input)?;
    let guest_map = guest_file.map()?;
    let guest_obj = Object::new(guest_map.as_ref())?;

    let guest_hull = guest_obj.segments().load_convex_hull()?;
    let mut output = Writer::new(&args.output, 0o755)?;
    relink_stage1(guest_hull, &mut output)?;

    Ok(())
}

fn relink_stage1(guest_hull: Range<u64>, writer: &mut Writer) -> Result<(), Error> {
    let obj = Object::new(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/embeds/libstage1.so"
    )))?;

    let hull = obj.segments().load_convex_hull()?;
    assert_eq!(hull.start, 0, "state1 must be relocatable");

    let base_offset = if guest_hull.start == 0 {
        0x800000
    } else {
        guest_hull.start
    };
    println!("Picked base_offset 0x{:x}", base_offset);

    let hull = (hull.start + base_offset)..(hull.end + base_offset);
    println!("Stage1 hull: {:x?}", hull);
    println!(" Guest hull: {:x?}", hull);

    let mut mapped = MappedObject::new(&obj, None)?;
    println!("Loaded stage1");

    Ok(())
}
