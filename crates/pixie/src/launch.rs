use core::arch::asm;

use crate::syscall;

/// # Safety
/// BIG DANGEROUS
#[inline(never)]
pub unsafe fn launch(stack_top: *mut u8, entry_point: u64) {
    unsafe {
        syscall::dup(0);

        asm!(
            "mov rsp, r12",
            "jmp r13",
            in("r12") stack_top,
            in("r13") entry_point,
            options(noreturn)
        );
    }
}
