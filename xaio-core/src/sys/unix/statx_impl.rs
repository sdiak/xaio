use std::io::{Error, ErrorKind, Result};

use crate::sys::StatX;

pub(crate) fn fstatat(
    dirfd: libc::c_int,
    pathname: *const i8,
    buf: &mut StatX,
    empty_path: bool,
    sym_nofollow: bool,
) -> Result<()> {
    let mut tmp_but: libc::stat = unsafe { std::mem::zeroed() };
    let status;
    cfg_if::cfg_if! {
        if #[cfg(not(any(target_os = "openbsd", target_os = "ios")))] {
            let mut flags = 0 as libc::c_int;
            if empty_path {
                flags |= libc::AT_EMPTY_PATH;
            }
            if sym_nofollow {
                flags |= libc::AT_SYMLINK_NOFOLLOW;
            }
            status = unsafe { libc::fstatat(dirfd, pathname, &mut tmp_but as _, flags) };
        } else {
            if pathname[0] == 0i8 && empty_path {
                status = libc::fstat(dirfd, &mut tmp_but as _);
            } else if athname[0] != '/' as _ {
                status = super::ioutils::with_dir(dirfd, || unsafe { libc::stat(pathname, &mut tmp_but as _) });
            }
        }
    }
    if status >= 0 {
        statx_from_stat(buf, &tmp_but);
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

pub(crate) fn statx(
    mut dirfd: libc::c_int,
    pathname: *const i8,
    mask: crate::sys::StatXWant,
    empty_path: bool,
    sym_nofollow: bool,
) -> Result<StatX> {
    if dirfd < 0 {
        dirfd = libc::AT_FDCWD;
    }
    let mut buf: StatX = unsafe { std::mem::zeroed() };
    cfg_if::cfg_if! {
        if #[cfg(target_os = "linux")] {
            let mut flags = 0 as libc::c_int;
            if empty_path {
                flags |= libc::AT_EMPTY_PATH;
            }
            if sym_nofollow {
                flags |= libc::AT_SYMLINK_NOFOLLOW;
            }
            let status = unsafe {libc::statx(dirfd, pathname, flags, mask.bits(), &mut buf as *mut StatX  as _)};
            if status < 0 {
                return Err(Error::last_os_error());
            }
        } else {
            fstatat(dirfd, pathname, buf, empty_path, sym_nofollow)?;
        }
    }
    Ok(buf)
}

fn statx_from_stat(dst: &mut super::statx_impl::StatX, src: &libc::stat) {
    use crate::sys::{StatXTimestamp, StatXWant};
    dst.blksize = src.st_blksize as _;
    dst.nlink = src.st_nlink as _;
    dst.uid = src.st_uid as _;
    dst.gid = src.st_gid as _;
    dst.mode = src.st_mode as _;
    dst.ino = src.st_ino as _;
    dst.size = src.st_size as _;
    dst.blocks = src.st_blocks as _;

    dst.rdev_major = src.st_rdev.wrapping_shr(32) as _;
    dst.rdev_minor = (src.st_rdev & 0xFFFFFFFF) as _;
    dst.dev_major = src.st_dev.wrapping_shr(32) as _;
    dst.dev_minor = (src.st_dev & 0xFFFFFFFF) as _;

    dst.mask = StatXWant::basic_stats();
    cfg_if::cfg_if! {
        if #[cfg(any(target_os = "macos", target_os = "ios"))] {
            dst.atime = StatXTimestamp{sec: src.st_atimespec.tv_sec as _    , nsec: src.st_atimespec.tv_nsec as _     };
            dst.mtime = StatXTimestamp{sec: src.st_mtimespec.tv_sec as _    , nsec: src.st_mtimespec.tv_nsec as _     };
            dst.ctime = StatXTimestamp{sec: src.st_ctimespec.tv_sec as _    , nsec: src.st_ctimespec.tv_nsec as _     };
            dst.btime = StatXTimestamp{sec: src.st_birthtimespec.tv_sec as _, nsec: src.st_birthtimespec.tv_nsec as _ };
            dst.mask |= StatXWant::BTIME;
        } else if #[cfg(target_os = "android")] {
            dst.atime = StatXTimestamp{sec: src.st_atime.tv_sec as _, nsec: src.st_atime.nsec as _ };
            dst.mtime = StatXTimestamp{sec: src.st_mtime.tv_sec as _, nsec: src.st_mtime.nsec as _ };
            dst.ctime = StatXTimestamp{sec: src.st_ctime.tv_sec as _, nsec: src.st_ctime.nsec as _ };
        } else if #[cfg(any(target_os = "freebsd", target_os = "dragonfly", target_os = "openbsd", target_os = "netbsd"))] {
            dst.atime = StatXTimestamp{sec: src.st_atim.tv_sec as _, nsec: src.st_atim.tv_nsec as _};
            dst.mtime = StatXTimestamp{sec: src.st_mtim.tv_sec as _, nsec: src.st_mtim.tv_nsec as _};
            dst.ctime = StatXTimestamp{sec: src.st_ctim.tv_sec as _, nsec: src.st_ctim.tv_nsec as _};
            if #[cfg(any(target_os = "freebsd", target_os = "netbsd"))] {
                dst.btime = StatXTimestamp{sec: src.st_birthtim.tv_sec as _, nsec: src.st_birthtim.tv_nsec as _};
                dst.mask |= StatXWant::BTIME;
            }
        } else {
          dst.atime = StatXTimestamp{sec: src.st_atime as _, nsec: 0 as _};
          dst.mtime = StatXTimestamp{sec: src.st_mtime as _, nsec: 0 as _};
          dst.ctime = StatXTimestamp{sec: src.st_ctime as _, nsec: 0 as _};
        }
    }
}
