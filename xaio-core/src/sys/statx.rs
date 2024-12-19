#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StatXTimestamp {
    /// Seconds elapsed since EPOCH
    sec: u64,
    /// Nanoseconds afer `sec`
    nsec: u32, /* Nanoseconds since tv_sec */
}

/// Want file type
const STATX_TYPE: libc::c_uint = 1u32 << 0;
/// Want file mode
const STATX_MODE: libc::c_uint = 1u32 << 1;
/// Want file nlink
const STATX_NLINK: libc::c_uint = 1u32 << 2;
/// Want uid
const STATX_UID: libc::c_uint = 1u32 << 3;
/// Want gid
const STATX_GID: libc::c_uint = 1u32 << 4;
/// Want atime
const STATX_ATIME: libc::c_uint = 1u32 << 5;
/// Want mtime
const STATX_MTIME: libc::c_uint = 1u32 << 6;
/// Want ctime
const STATX_CTIME: libc::c_uint = 1u32 << 7;
/// Want ino
const STATX_INO: libc::c_uint = 1u32 << 8;
/// Want size
const STATX_SIZE: libc::c_uint = 1u32 << 9;
/// Want blocks
const STATX_BLOCKS: libc::c_uint = 1u32 << 10;
/// Want btime
const STATX_BTIME: libc::c_uint = 1u32 << 11;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Want: libc::c_uint {
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
impl Want {
    /// Same as `TYPE | MODE | NLINK | UID | GID | ATIME | MTIME | CTIME | INO | SIZE | BLOCKS`
    pub const fn basic_stats() -> Want {
        Want::from_bits_retain(
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
    pub const fn all_stats() -> Want {
        Want::from_bits_retain(libc::c_uint::MAX)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StatX {
    /// Mask indicating the filled fields
    pub mask: u32,
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
    /// Mask to show what's supported in stx_attributes
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
    pub fn do_something() -> i32 {
        42
    }
}
