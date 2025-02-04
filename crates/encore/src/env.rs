#![allow(unsafe_op_in_unsafe_fn)]
use core::fmt;

use alloc::vec::Vec;

use crate::utils::NulTerminated;

#[repr(C)]
pub struct Auxv {
    pub typ: AuxvType,
    pub value: u64,
}

impl fmt::Debug for Auxv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AT_{:?} = 0x{:x}", self.typ, self.value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct AuxvType(u64);

impl AuxvType {
    pub const NULL: Self = Self(0);
    pub const PHDR: Self = Self(3);
    pub const PHNUM: Self = Self(5);
    pub const BASE: Self = Self(7);
    pub const ENTRY: Self = Self(9);
}

impl fmt::Debug for AuxvType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            Self::PHDR => "PHDR",
            Self::PHNUM => "PHNUM",
            Self::BASE => "BASE",
            Self::ENTRY => "ENTRY",
            _ => "(UNKNOWN)",
        })
    }
}

#[derive(Default)]
pub struct Env {
    /// Auxiliary vectors
    pub vectors: Vec<&'static mut Auxv>,

    /// CLI args
    pub args: Vec<&'static str>,

    /// Env variables
    pub vars: Vec<&'static str>,
}

impl Env {
    /// # Safety
    /// Walking the stack, not exactly the safest thing
    pub unsafe fn read(stack_top: *mut u8) -> Self {
        let mut ptr: *mut u64 = stack_top as _;

        let mut env = Self::default();

        // Read in the cli Arguments
        ptr = ptr.add(1);
        while *ptr != 0 {
            let arg = (*ptr as *const u8).cstr();
            env.args.push(arg);
            ptr = ptr.add(1);
        }

        // Read in env variables
        ptr = ptr.add(1);
        while *ptr != 0 {
            let var = (*ptr as *const u8).cstr();
            env.vars.push(var);
            ptr = ptr.add(1);
        }

        // Read in auxiliary vectors
        ptr = ptr.add(1);
        let mut ptr: *mut Auxv = ptr as _;
        while (*ptr).typ != AuxvType::NULL {
            env.vectors.push(ptr.as_mut().unwrap());
            ptr = ptr.add(1);
        }

        env
    }

    /// Finds an auxiliary vector by type
    /// Panics if the vector cannot be found
    pub fn find_vector(&mut self, typ: AuxvType) -> &mut Auxv {
        self.vectors
            .iter_mut()
            .find(|v| v.typ == typ)
            .unwrap_or_else(|| panic!("aux vector {:?} not found", typ))
    }
}
