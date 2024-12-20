cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub use libc::{STATX_TYPE, STATX_MODE, STATX_NLINK, STATX_UID, STATX_GID, STATX_ATIME, STATX_MTIME, STATX_CTIME,
            STATX_INO, STATX_SIZE, STATX_BLOCKS, STATX_BTIME, STATX_MNT_ID, STATX_DIOALIGN };
    } else {
        const STATX_TYPE: libc::c_uint = 1u32 << 0;
        const STATX_MODE: libc::c_uint = 1u32 << 1;
        const STATX_NLINK: libc::c_uint = 1u32 << 2;
        const STATX_UID: libc::c_uint = 1u32 << 3;
        const STATX_GID: libc::c_uint = 1u32 << 4;
        const STATX_ATIME: libc::c_uint = 1u32 << 5;
        const STATX_MTIME: libc::c_uint = 1u32 << 6;
        const STATX_CTIME: libc::c_uint = 1u32 << 7;
        const STATX_INO: libc::c_uint = 1u32 << 8;
        const STATX_SIZE: libc::c_uint = 1u32 << 9;
        const STATX_BLOCKS: libc::c_uint = 1u32 << 10;
        const STATX_BTIME: libc::c_uint = 1u32 << 11;
        const STATX_MNT_ID: libc::c_uint = 1u32 << 12;
        const STATX_DIOALIGN: libc::c_uint = 1u32 << 13;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StatXTimestamp {
    /// Seconds elapsed since EPOCH
    pub sec: u64,
    /// Nanoseconds after `sec`
    pub nsec: u32,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct StatXWant: libc::c_uint {
        /// Want file type
        const TYPE = STATX_TYPE;
        /// Want file mode
        const MODE = STATX_MODE;
        /// Want file nlink
        const NLINK = STATX_NLINK;
        /// Want uid
        const UID = STATX_UID;
        /// Want gid
        const GID = STATX_GID;
        /// Want atime
        const ATIME = STATX_ATIME;
        /// Want mtime
        const MTIME = STATX_MTIME;
        /// Want ctime
        const CTIME = STATX_CTIME;
        /// Want ino
        const INO = STATX_INO;
        /// Want size
        const SIZE = STATX_SIZE;
        /// Want blocks
        const BLOCKS = STATX_BLOCKS;
        /// Want btime
        const BTIME = STATX_BTIME;
    }
}
impl StatXWant {
    /// Same as `TYPE | MODE | NLINK | UID | GID | ATIME | MTIME | CTIME | INO | SIZE | BLOCKS`
    pub const fn basic_stats() -> StatXWant {
        StatXWant::from_bits_retain(
            STATX_TYPE
                | STATX_MODE
                | STATX_NLINK
                | STATX_UID
                | STATX_GID
                | STATX_ATIME
                | STATX_MTIME
                | STATX_CTIME
                | STATX_INO
                | STATX_SIZE
                | STATX_BLOCKS,
        )
    }
    /// All field available on current system
    pub const fn all_stats() -> StatXWant {
        StatXWant::from_bits_retain(libc::c_uint::MAX)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StatX {
    /// Mask indicating the filled fields
    pub mask: StatXWant,
    /// Block size for I/O
    pub blksize: u32,
    /// Extra file attributes
    pub attributes: u64,
    /// The number of hard links
    pub nlink: u32,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// Type and mode
    pub mode: u16,
    /// Inode number
    pub ino: u64,
    /// Size in bytes
    pub size: u64,
    /// Number of 512 bytes blocks allocated
    pub blocks: u64,
    /// Mask showing what's supported in attributes
    pub attributes_mask: u64,
    /// Last access timestamp
    pub atime: StatXTimestamp,
    /// Creation timestamp
    pub btime: StatXTimestamp,
    /// Last status change timestamp
    pub ctime: StatXTimestamp,
    /// Last modification timestamp
    pub mtime: StatXTimestamp,
    /// Major device id (when the file is a device)
    pub rdev_major: u32,
    /// Minor device id (when the file is a device)
    pub rdev_minor: u32,
    /// Major device id of the fs device where this file is stored
    pub dev_major: u32,
    /// Minor device id of the fs device where this file is stored
    pub dev_minor: u32,
    /// Mount identifier
    pub mnt_id: u64,
    /// Alignement of memory for direct IO
    pub dio_mem_align: u32,
    /// Alignement of offset for direct IO
    pub dio_offset_align: u32,
    /// Reserved
    pub _reserved: [u64; 12],
}

impl StatX {
    // pub fn new() -> Result<StatX> {
    //     super::statix()
    // }
}
