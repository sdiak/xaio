use crate::stat::FileStat;
use crate::Status;

#[cfg(all(target_os = "linux", any(target_env = "gnu", target_os = "android")))]
pub fn statx(
    result: &mut FileStat,
    dirfd: libc::c_int,
    pathname: &[u8],
    flags: libc::c_int,
    mask: u32,
) -> Status {
    use std::mem::MaybeUninit;

    // let x = std::fs::metadata("/tmp")?;

    let mut buf = MaybeUninit::<libc::statx>::uninit();
    if unsafe {
        libc::statx(
            dirfd,
            pathname.as_ptr() as *const i8,
            flags,
            mask,
            buf.as_mut_ptr(),
        )
    } >= 0
    {
        let buf = unsafe { buf.assume_init() };
        result.mask = buf.stx_mask;
        result.blksize = buf.stx_blksize;
        result.attributes_mask = buf.stx_attributes_mask;
        result.attributes = buf.stx_attributes;
        result.nlink = buf.stx_nlink;
        result.uid = buf.stx_uid;
        result.gid = buf.stx_gid;
        result.mode = buf.stx_mode;
        result.ino = buf.stx_ino;
        result.size = buf.stx_size;
        result.blocks = buf.stx_blocks;
        result.dev.major = buf.stx_dev_major;
        result.dev.minor = buf.stx_dev_minor;
        result.rdev.major = buf.stx_rdev_major;
        result.rdev.minor = buf.stx_rdev_minor;
        result.mnt_id = buf.stx_mnt_id;
        result.flags = 0;
        result.gen = 0;
        result.dio_mem_align = buf.stx_dio_mem_align;
        result.dio_offset_align = buf.stx_dio_offset_align;
        result.atim.sec = buf.stx_atime.tv_sec;
        result.atim.nsec = buf.stx_atime.tv_nsec;
        result.mtim.sec = buf.stx_atime.tv_sec;
        result.mtim.nsec = buf.stx_mtime.tv_nsec;
        result.ctim.sec = buf.stx_atime.tv_sec;
        result.ctim.nsec = buf.stx_ctime.tv_nsec;
        result.btim.sec = buf.stx_atime.tv_sec;
        result.btim.nsec = buf.stx_btime.tv_nsec;
        Status::new(0)
    } else {
        Status::last_os_error()
    }
}
