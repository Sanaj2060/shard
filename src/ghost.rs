use fuser::{Filesystem, ReplyAttr, ReplyEntry, ReplyOpen, ReplyWrite, ReplyDirectory, Request, FileType, FileAttr};
use std::ffi::OsStr;
use std::time::{Duration, SystemTime};
use libc::ENOENT;
use crate::buffer::AtomicBuffer;
use crate::wal::Wal;
use std::sync::Arc;
use tokio::runtime::Handle;

const TTL: Duration = Duration::from_secs(1);

// Fake Directory Attributes
const DIR_ATTR: FileAttr = FileAttr {
    ino: 1, size: 0, blocks: 0, atime: SystemTime::UNIX_EPOCH, mtime: SystemTime::UNIX_EPOCH, ctime: SystemTime::UNIX_EPOCH, crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::Directory, perm: 0o777, nlink: 2, uid: 501, gid: 20, rdev: 0, blksize: 512, flags: 0,
};

// Fake File Attributes (Black Hole)
const FILE_ATTR: FileAttr = FileAttr {
    ino: 2, size: 0, blocks: 0, atime: SystemTime::UNIX_EPOCH, mtime: SystemTime::UNIX_EPOCH, ctime: SystemTime::UNIX_EPOCH, crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::RegularFile, perm: 0o666, nlink: 1, uid: 501, gid: 20, rdev: 0, blksize: 512, flags: 0,
};

pub struct ShardGhost {
    pub buffer: Arc<AtomicBuffer>,
    pub wal: Arc<Wal>,
    pub rt_handle: Handle,
}

impl Filesystem for ShardGhost {
    fn lookup(&mut self, _req: &Request, parent: u64, _name: &OsStr, reply: ReplyEntry) {
        if parent == 1 {
            // Allow any file to be "created/found" in this virtual directory
            reply.entry(&TTL, &FILE_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &DIR_ATTR),
            _ => reply.attr(&TTL, &FILE_ATTR),
        }
    }

    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: ReplyOpen) {
        reply.opened(0, 0);
    }

    fn write(&mut self, _req: &Request, _ino: u64, _fh: u64, _offset: i64, data: &[u8], _write_flags: u32, _flags: i32, _lock_owner: Option<u64>, reply: ReplyWrite) {
        // Intercept data streams
        let wal = self.wal.clone();
        let data_clone = data.to_vec(); 

        // 1. Hardened WAL persistence (blocking so FUSE knows it's safe)
        let _ = self.rt_handle.block_on(async move {
            wal.append(&data_clone).await
        });

        // 2. Format and Buffer
        let mut final_data = data.to_vec();
        if !final_data.ends_with(b"\n") {
            final_data.push(b'\n'); // Ensure JSON records don't fuse on the same line
        }

        if self.buffer.write(&final_data).is_ok() {
            // Tell the OS the write succeeded so it drops the file
            reply.written(data.len() as u32);
        } else {
            reply.error(libc::ENOSPC); // Out of space
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino != 1 {
            reply.error(libc::ENOTDIR);
            return;
        }
        if offset == 0 {
            let _ = reply.add(1, 0, FileType::Directory, ".");
            let _ = reply.add(1, 1, FileType::Directory, "..");
        }
        reply.ok();
    }

    fn create(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &std::ffi::OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        // Return a virtual file attribute to the kernel so it thinks the file was created
        reply.created(&TTL, &FILE_ATTR, 0, 0, 0);
    }

    fn setattr(
        &mut self,
        _req: &Request,
        _ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>, // Corrected from TimeOrNow
        _fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        // Just echo back the standard file attributes
        reply.attr(&TTL, &FILE_ATTR);
    }
}