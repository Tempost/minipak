use crate::{
    error::EncoreError,
    syscall::{self, FileDescriptor, MmapFlags, MmapProtection},
};

pub struct MmapOptions {
    protection: MmapProtection,
    flags: MmapFlags,
    len: u64,
    file: Option<FileOptions>,
    at: Option<u64>,
}

#[derive(Default, Clone)]
pub struct FileOptions {
    /// An open file descriptor
    pub fd: FileDescriptor,
    /// Offset of where to map the file
    pub offset: u64,
}

impl MmapOptions {
    pub fn new(len: u64) -> Self {
        Self {
            protection: MmapProtection::READ | MmapProtection::WRITE,
            flags: MmapFlags::ANONYMOUS | MmapFlags::PRIVATE,
            len,
            file: None,
            at: None,
        }
    }

    pub fn file(&mut self, file: FileOptions) -> &mut Self {
        self.file = Some(file);
        self
    }

    pub fn protection(&mut self, protection: MmapProtection) -> &mut Self {
        self.protection = protection;
        self
    }

    pub fn flags(&mut self, flags: MmapFlags) -> &mut Self {
        self.flags = flags;
        self
    }

    pub fn at(&mut self, at: u64) -> &mut Self {
        self.at = Some(at);
        self
    }

    pub fn map(&mut self) -> Result<u64, EncoreError> {
        let mut flags = self.flags;

        if let Some(at) = &self.at {
            if !is_aligned(*at) {
                return Err(EncoreError::MmapMemUnaligned(*at));
            }
            flags.insert(MmapFlags::FIXED);
        }

        if let Some(file) = &self.file {
            if !is_aligned(file.offset) {
                return Err(EncoreError::MmapFileUnaligned(file.offset));
            }
            flags.remove(MmapFlags::ANONYMOUS);
        }

        let file = self.file.clone().unwrap_or_default();
        let addr = self.at.unwrap_or_default();

        let res =
            unsafe { syscall::mmap(addr, self.len, self.protection, flags, file.fd, file.offset) };
        if res as i64 == -1 {
            return Err(EncoreError::MmapFailed);
        }

        Ok(res)
    }
}

fn is_aligned(x: u64) -> bool {
    x & 0xFFF == 0
}
