#![allow(unsafe_op_in_unsafe_fn)]
use core::fmt;

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        {
            use ::core::fmt::Write;
            ::core::writeln!($crate::utils::Stdout, $($arg)*).ok();
        }
    }
}

pub struct Stdout;
impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            crate::syscall::write(
                crate::syscall::FileDescriptor::STDOUT,
                s.as_ptr(),
                s.len() as _,
            );
        }

        Ok(())
    }
}

pub trait NulTerminated
where
    Self: Sized,
{
    /// Turns a pointer into a byte slice,
    /// assuming it finds a null terminator.
    ///
    /// # Safety
    /// Dereferences an arbitrary pointer.
    unsafe fn null_terminated(self) -> &'static [u8];

    /// Turns self into a string.
    ///
    /// # Safety
    /// Dereferences an arbitrary pointer.
    unsafe fn cstr(self) -> &'static str {
        core::str::from_utf8(self.null_terminated()).unwrap()
    }
}

impl NulTerminated for *const u8 {
    unsafe fn null_terminated(self) -> &'static [u8] {
        let mut j = 0;
        while *self.add(j) != 0 {
            j += 1;
        }
        core::slice::from_raw_parts(self, j)
    }
}
