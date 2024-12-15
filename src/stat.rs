#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileStatTimestamp {
    pub sec: i64,
    pub nsec: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileStatDev {
    pub major: u32,
    pub minor: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileStat {
    /// Bits indication filled fields
    pub mask: u32,
    /// Block size
    pub blksize: u32,
    /// File attributes mask
    pub attributes_mask: u64,
    /// File attributes
    pub attributes: u64,
    /// Number of hard links
    pub nlink: u32,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// File type and mode
    pub mode: u16,

    _reserved_2_: u16,
    /// Inode number
    pub ino: u64,
    /// Total size (bytes)
    pub size: u64,
    /// Number of 512-bytes blocks
    pub blocks: u64,
    /// Device identifier (when the file is a device)
    pub dev: FileStatDev,
    /// Device identifier for the device containing the filesystem owning the file
    pub rdev: FileStatDev,
    /// Mount identifier
    pub mnt_id: u64,
    /// Direct I/O user buffer alignement
    pub dio_mem_align: u32,
    /// Direct I/O file offset alignement
    pub dio_offset_align: u32,
    /// File last access time
    pub atim: FileStatTimestamp,
    /// File last modification time
    pub mtim: FileStatTimestamp,
    /// File last status change time
    pub ctim: FileStatTimestamp,
    /// File creation time
    pub btim: FileStatTimestamp,

    _reverved_80_: [u8; 80],
    /// User defined flags for the file
    pub flags: u64,
    /// Generation number for the file
    pub gen: u64,
}
