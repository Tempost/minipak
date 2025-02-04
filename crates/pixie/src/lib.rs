#![no_std]
extern crate alloc;

use core::{
    cmp::{max, min},
    ops::Range,
};

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

        let mut map_opts = MmapOptions::new(mem_len);
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
}
