use alloc::string::String;

#[derive(Debug)]
pub enum EncoreError {
    /// Could not open file `0`
    Open(String),
    /// Could not write file `0`
    Write(String),
    /// Could not statfile `0`
    Stat(String),

    /// mmap fixed address provided was not aligned to 0x1000: {0}
    MmapMemUnaligned(u64),
    /// mmap file offset provided was not aligned to 0x1000: {0}
    MmapFileUnaligned(u64),
    /// Failed syscall
    MmapFailed,
}
