#![no_std]
extern crate alloc;

use core::{
    cmp::{max, min},
    ops::Range,
};

use alloc::boxed::Box;
pub use deku;
use deku::{DekuContainerRead, DekuError};
use encore::prelude::*;

mod manifest;
pub use manifest::*;

mod writer;
pub use writer::*;

mod format;
pub use format::*;

mod launch;
pub use launch::*;

#[derive(displaydoc::Display, Debug)]
/// A pixie error
pub enum PixieError {
    /// `{0}`
    Deku(DekuError),
    /// `{0}`
    Encore(EncoreError),

    /// no segments found
    NoSegmentsFound,
    /// could not find segment of type `{0:?}`
    SegmentNotFound(SegmentType),

    /// cannot map non-relocatable object at fixed position
    CannotMapNonRelocatableObjectAtFixedPosition,

    /// cannot relocate non-relocatable object
    CannotRelocateNonRelocatableObject,

    /// could not find dynamic entry of type `{0:?}`
    DynamicEntryNotFound(DynamicTagType),

    /// unsuported relocation type `{0:?}`
    UnsuportedRela(Rela),
}

impl From<DekuError> for PixieError {
    fn from(e: DekuError) -> Self {
        Self::Deku(e)
    }
}

impl From<EncoreError> for PixieError {
    fn from(e: EncoreError) -> Self {
        Self::Encore(e)
    }
}

pub struct Object<'a> {
    header: ObjectHeader,
    slice: &'a [u8],
    segments: Segments<'a>,
}

impl<'a> Object<'a> {
    pub fn new(slice: &'a [u8]) -> Result<Self, PixieError> {
        let input = (slice, 0);
        let (_, header) = ObjectHeader::from_bytes(input)?;
        let segments = {
            let mut segments = Segments::default();
            let mut input = (&slice[header.ph_offset as usize..], 0);
            for _ in 0..header.ph_count {
                let (rest, ph) = ProgramHeader::from_bytes(input)?;
                segments.items.push(Segment::new(ph, slice));
                input = rest;
            }
            segments
        };

        Ok(Self {
            slice,
            segments,
            header,
        })
    }

    /// Read all dynamic entries
    pub fn read_dynamic_entries(&'a self) -> Result<DynamicEntries<'a>, PixieError> {
        let dyn_seg = self.segments().find(SegmentType::Dynamic)?;
        let mut entries = DynamicEntries::default();
        let mut input = (dyn_seg.slice(), 0);
        loop {
            let (rest, tag) = DynamicTag::from_bytes(input)?;
            if tag.typ == DynamicTagType::Null {
                break;
            }
            entries.items.push(DynamicEntry {
                tag,
                full_slice: &self.slice(),
            });
            input = rest;
        }

        Ok(entries)
    }

    pub fn header(&self) -> &ObjectHeader {
        &self.header
    }

    pub fn slice(&self) -> &[u8] {
        &self.slice
    }

    pub fn segments(&self) -> &Segments {
        &self.segments
    }
}

/// Segment as read from an ELF file
pub struct Segment<'a> {
    /// Program header for this segment
    header: ProgramHeader,
    /// Slice for this segment (not the whole ELF file)
    slice: &'a [u8],
}

impl<'a> Segment<'a> {
    ///Instantiate a segment
    fn new(header: ProgramHeader, full_slice: &'a [u8]) -> Self {
        let start = header.offset as usize;
        let len = header.filesz as usize;
        Segment {
            header,
            slice: &full_slice[start..][..len],
        }
    }

    /// Returns the segment's type
    pub fn typ(&self) -> SegmentType {
        self.header.typ
    }

    /// Returns slice of the segment
    pub fn slice(&self) -> &[u8] {
        &self.slice
    }

    /// Returns the segment's type
    pub fn header(&self) -> &ProgramHeader {
        &self.header
    }
}

/// A collection of segments
#[derive(Default)]
pub struct Segments<'a> {
    items: Vec<Segment<'a>>,
}

impl<'a> Segments<'a> {
    pub fn all(&self) -> &[Segment] {
        &self.items
    }

    /// Returns all segments of a certain type
    pub fn of_type(&self, typ: SegmentType) -> impl Iterator<Item = &Segment<'a>> + '_ {
        self.items.iter().filter(move |s| s.typ() == typ)
    }

    /// Returns the first segment of a given type or non if none matched
    pub fn find(&self, typ: SegmentType) -> Result<&Segment, PixieError> {
        self.of_type(typ)
            .next()
            .ok_or(PixieError::SegmentNotFound(typ))
    }

    ///Returns a 4k-aligned convex hull of all the load segments
    pub fn load_convex_hull(&self) -> Result<Range<u64>, PixieError> {
        let hull = self
            .of_type(SegmentType::Load)
            .map(|s| s.header().mem_range())
            .reduce(|a, b| min(a.start, b.start)..max(a.end, b.end))
            .ok_or(PixieError::NoSegmentsFound)?;
        Ok(hull)
    }
}

/// An ELF object mapped into memory
pub struct MappedObject<'a> {
    object: &'a Object<'a>,
    hull: Range<u64>,
    base_offset: u64,
    mem: &'a mut [u8],
}

impl<'a> MappedObject<'a> {
    pub fn new(object: &'a Object, mut at: Option<u64>) -> Result<Self, PixieError> {
        let hull = object.segments().load_convex_hull()?;
        let is_relocatable = hull.start == 0;

        if !is_relocatable {
            // non-relocatable object, we need to map it at its fixed position
            if at.is_some() {
                return Err(PixieError::CannotMapNonRelocatableObjectAtFixedPosition);
            }
            at = Some(hull.start)
        }
        let mem_len = hull.end - hull.start;

        let mut map_opts = MmapOptions::new(hull.end - hull.start);
        // TODO: Make it so the map_opts reflect what the segment actually requires
        map_opts.protection(MmapProtection::READ | MmapProtection::WRITE | MmapProtection::EXEC);
        if let Some(at) = at {
            map_opts.at(at);
        }

        let res = map_opts.map()?;
        let base_offset = if is_relocatable { res } else { 0 };
        let mem = unsafe { core::slice::from_raw_parts_mut(res as _, mem_len as _) };

        let mut mapped = Self {
            hull,
            object,
            mem,
            base_offset,
        };
        mapped.copy_load_segments();
        Ok(mapped)
    }

    /// Apply relocations with the given base offset
    pub fn relocate(&mut self, base_offset: u64) -> Result<(), PixieError> {
        // ?????????? Stage1 starts a relocatable, then becomes not?
        if self.is_relocatable() {
            return Err(PixieError::CannotRelocateNonRelocatableObject);
        }

        let dyn_entries = self.object.read_dynamic_entries()?;
        let syms = dyn_entries.syms()?;

        let relas = dyn_entries
            .find(DynamicTagType::Rela)?
            .parse_all(dyn_entries.find(DynamicTagType::RelaSz)?);

        let plt_relas: Box<dyn Iterator<Item = _>> = match dyn_entries.find(DynamicTagType::JmpRel)
        {
            Ok(jmprel) => Box::new(jmprel.parse_all(dyn_entries.find(DynamicTagType::PltRelSz)?)),
            Err(_) => Box::new(core::iter::empty()) as _,
        };

        for rela in relas.chain(plt_relas) {
            let rela = rela?;
            self.apply_rela(&syms, &rela, base_offset)?;
        }

        Ok(())
    }

    /// Apply a relocation
    fn apply_rela(&mut self, syms: &Syms, rela: &Rela, base_offset: u64) -> Result<(), PixieError> {
        match rela.typ {
            RelType::_64 | RelType::GlobDat | RelType::JumpSlot | RelType::Relative => {}
            _ => {
                return Err(PixieError::UnsuportedRela(rela.clone()));
            }
        }

        let (sym, _) = syms.nth(rela.sym as _)?;
        let value = base_offset + sym.value + rela.addend;

        let mem_offset = self.vaddr_to_mem_offset(rela.offset);
        unsafe {
            let target = self.mem.as_ptr().add(mem_offset) as *mut u64;
            *target = value;
        }

        Ok(())
    }

    fn copy_load_segments(&mut self) {
        for seg in self.object.segments().of_type(SegmentType::Load) {
            let mem_start = self.vaddr_to_mem_offset(seg.header().vaddr);
            let dst = &mut self.mem[mem_start..][..seg.slice().len()];
            dst.copy_from_slice(seg.slice());
        }
    }

    /// Convert a vaddr to a memory offset
    pub fn vaddr_to_mem_offset(&self, vaddr: u64) -> usize {
        (vaddr - self.hull.start) as _
    }

    /// Returns a view of (potentially relocated) `mem` for a given range
    pub fn vaddr_slice(&self, range: Range<u64>) -> &[u8] {
        &self.mem[self.vaddr_to_mem_offset(range.start)..self.vaddr_to_mem_offset(range.end)]
    }

    /// Returns true if the object's base offset is zero, which we assume
    /// means it can be mapped anywhere.
    pub fn is_relocatable(&self) -> bool {
        self.base_offset == 0
    }

    /// Returns the offset between the object's base and where we loaded it
    pub fn base_offset(&self) -> u64 {
        self.base_offset
    }

    /// Returns the base address for this executable
    pub fn base(&self) -> u64 {
        self.mem.as_ptr() as _
    }

    /// Returns the (non-relocated) vaddr of a symbol by name
    pub fn lookup_sym(&self, name: &str) -> Result<Sym, PixieError> {
        let dyn_entries = self.object.read_dynamic_entries()?;
        dyn_entries.syms()?.by_name(name)
    }
}

#[derive(Default)]
pub struct DynamicEntries<'a> {
    items: Vec<DynamicEntry<'a>>,
}

impl<'a> DynamicEntries<'a> {
    /// Returns slice of all entries
    pub fn all(&self) -> &[DynamicEntry<'a>] {
        &self.items
    }

    /// Iterates over all entries of a given type
    pub fn of_type(&self, typ: DynamicTagType) -> impl Iterator<Item = &DynamicEntry<'a>> {
        self.items.iter().filter(move |entry| entry.typ() == typ)
    }

    /// Finds the first entry of a given type
    pub fn find(&self, typ: DynamicTagType) -> Result<&DynamicEntry<'a>, PixieError> {
        self.of_type(typ)
            .next()
            .ok_or(PixieError::DynamicEntryNotFound(typ))
    }

    /// Constructs an instance of `Syms`. Requires the presence of the `SymTab`,
    /// `SymEnt` and `StrTab` dynamic entries.
    pub fn syms(&'a self) -> Result<Syms<'a>, PixieError> {
        Ok(Syms {
            symtab: self.find(DynamicTagType::SymTab)?,
            syment: self.find(DynamicTagType::SymEnt)?,
            strtab: self.find(DynamicTagType::StrTab)?,
        })
    }
}

/// An entry in the `DYNAMIC` section
pub struct DynamicEntry<'a> {
    /// The dynamic tag as read from the `DYNAMIC` section
    tag: DynamicTag,

    /// A slice of the full ELF object
    full_slice: &'a [u8],
}

impl<'a> DynamicEntry<'a> {
    /// Returns the type of this dynamic entry
    pub fn typ(&self) -> DynamicTagType {
        self.tag.typ
    }

    /// Returns a slice of the full file starting with this entry interpreted as
    /// an offset.
    pub fn as_slice(&self) -> &'a [u8] {
        &self.full_slice[self.as_usize()..]
    }

    /// Returns this entry's value as an `usize`
    pub fn as_usize(&self) -> usize {
        self.as_u64() as usize
    }

    /// Returns this entry's value as an `u64`
    pub fn as_u64(&self) -> u64 {
        self.tag.addr
    }

    /// Parses several `T` records, using `self` at the start of the input, and
    /// `len` total length of the input.
    pub fn parse_all<T>(
        &self,
        len: &DynamicEntry<'a>,
    ) -> impl Iterator<Item = Result<T, PixieError>> + 'a
    where
        T: DekuContainerRead<'a>,
    {
        let slice = &self.as_slice()[..len.as_usize()];
        let mut input = (slice, 0);

        core::iter::from_fn(move || -> Option<Result<T, PixieError>> {
            if input.0.is_empty() {
                return None;
            }

            let (rest, t) = match T::from_bytes(input) {
                Ok(x) => x,
                Err(e) => return Some(Err(e.into())),
            };
            input = rest;
            Some(Ok(t))
        })
    }

    /// Parses the nth `T` record, using `self` as the start of the input, and
    /// `record_len` as the record length.
    pub fn parse_nth<T>(&self, record_len: &DynamicEntry<'a>, n: usize) -> Result<T, DekuError>
    where
        T: DekuContainerRead<'a>,
    {
        let slice = &self.as_slice()[(record_len.as_usize() * n)..];
        let input = (slice, 0);
        let (_, t) = T::from_bytes(input)?;
        Ok(t)
    }
}

pub struct Syms<'a> {
    /// Indicates the start of the symbol file
    symtab: &'a DynamicEntry<'a>,
    /// Indicates the size of a symbol entry
    syment: &'a DynamicEntry<'a>,
    /// Indicates the start of the string table
    strtab: &'a DynamicEntry<'a>,
}

impl<'a> Syms<'a> {
    /// Read the nth symbol
    pub fn nth(&self, n: usize) -> Result<(Sym, &'a str), PixieError> {
        let sym: Sym = self.symtab.parse_nth(&self.syment, n)?;
        let name = unsafe { self.strtab.as_slice().as_ptr().add(sym.name as _).cstr() };
        Ok((sym, name))
    }

    /// Find a symbol by name. Will end up panicking if the symbol
    /// is not found!
    pub fn by_name(&self, name: &str) -> Result<Sym, PixieError> {
        let mut i = 0;
        loop {
            let (sym, sym_name) = self.nth(i)?;
            if sym_name == name {
                return Ok(sym);
            }
            i += 1;
        }
    }
}

/// Align *down* to the nearest 4k boundary
pub fn floor(val: u64) -> u64 {
    val & !0xFFF
}

/// Align *up* to the nearest 4k boundary
pub fn ceil(val: u64) -> u64 {
    if floor(val) == val {
        val
    } else {
        floor(val + 0x1000)
    }
}

/// Given a convex hul, align its start *down* to nearest 4k boundary and
/// its end *up* to the nearest 4k boundary
pub fn align_hull(hull: Range<u64>) -> Range<u64> {
    floor(hull.start)..ceil(hull.end)
}
