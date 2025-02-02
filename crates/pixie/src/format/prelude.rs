use core::fmt;

pub(crate) use alloc::{format, vec::Vec};
pub(crate) use deku::prelude::*;
pub(crate) use deku::{DekuContainerRead, DekuRead};

pub fn hex_fmt<T: fmt::Debug>(n: &T, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "0x{:2x?}", n)
}
