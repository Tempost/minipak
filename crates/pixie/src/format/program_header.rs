use super::prelude::*;
use custom_debug::Debug;

#[derive(Debug, DekuRead, DekuWrite, Clone)]
pub struct ProgramHeader {
    pub typ: SegmentType,

    #[debug(with = "hex_fmt")]
    pub flags: u32,
    #[debug(with = "hex_fmt")]
    pub offset: u64,
    #[debug(with = "hex_fmt")]
    pub vaddr: u64,
    #[debug(with = "hex_fmt")]
    pub paddr: u64,
    #[debug(with = "hex_fmt")]
    pub filesz: u64,
    #[debug(with = "hex_fmt")]
    pub memsz: u64,
    #[debug(with = "hex_fmt")]
    pub align: u64,
}

#[derive(Debug, DekuRead, DekuWrite, Clone, Copy, PartialEq)]
#[deku(id_type = "u32")]
pub enum SegmentType {
    #[deku(id = "0x0")]
    Null,
    #[deku(id = "0x1")]
    Load,
    #[deku(id = "0x2")]
    Dynamic,
    #[deku(id = "0x3")]
    Interp,
    #[deku(id = "0x4")]
    Note,
    #[deku(id = "0x5")]
    ShLib,
    #[deku(id = "0x6")]
    PHdr,
    #[deku(id = "0x7")]
    Tls,
    #[deku(id = "0x60000000")]
    LoOS,
    #[deku(id = "0x6fffffff")]
    HiOS,
    #[deku(id = "0x70000000")]
    LoProc,
    #[deku(id = "0x7fffffff")]
    HiProc,
    #[deku(id = "0x6474E550")]
    GnuEhFrame,
    #[deku(id = "0x6474e551")]
    GnuStack,
    #[deku(id = "0x6000_000")]
    GnuRelRo,
    #[deku(id = "0x6000_000")]
    GnuProperty,
    #[deku(id_pat = "_")]
    Other(u32),
}

impl ProgramHeader {
    pub const SIZE: u16 = 64;
    pub const EXECUTE: u32 = 1;
    pub const WRITE: u32 = 2;
    pub const READ: u32 = 4;

    /// Returns a range that spans from offset to offset+filesz
    pub fn file_range(&self) -> core::ops::Range<usize> {
        let start = self.offset as usize;
        let len = self.filesz as usize;
        let end = start + len;
        start..end
    }

    /// Returns a range that spans from vaddr to vaddr+memsz
    pub fn mem_range(&self) -> core::ops::Range<u64> {
        let start = self.vaddr as u64;
        let len = self.memsz as u64;
        let end = start + len;
        start..end
    }
}
