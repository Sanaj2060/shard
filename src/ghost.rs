use fuser::{Filesystem, ReplyAttr, ReplyEntry, ReplyOpen, ReplyWrite, ReplyDirectory, Request, FileType, FileAttr};
use std::ffi::OsStr;
use std::time::{Duration, SystemTime};
use crate::buffer::AtomicBuffer;
use crate::wal::Wal;
use std::sync::Arc;
use tokio::runtime::Handle;

const TTL: Duration = Duration::from_secs(1);
const DIR_ATTR: FileAttr = FileAttr {
    ino: 1, size: 0, blocks: 0, atime: SystemTime::UNIX_EPOCH, mtime: SystemTime::UNIX_EPOCH, ctime: SystemTime::UNIX_EPOCH, crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::Directory, perm: 0o777, nlink: 2, uid: 1000, gid: 1000, rdev: 0, blksize: 512, flags: 0,
};

const FILE_ATTR: FileAttr = FileAttr {
    ino: 2, size: 0, blocks: 0, atime: SystemTime::UNIX_EPOCH, mtime: SystemTime::UNIX_EPOCH, ctime: SystemTime::UNIX_EPOCH, crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::RegularFile, perm: 0o666, nlink: 1, uid: 1000, gid: 1000, rdev: 0, blksize: 512, flags: 0,
};

pub struct ShardGhost {
    pub buffer: Arc<AtomicBuffer>,
    pub wal: Arc<Wal>,
    pub rt_handle: Handle,
}

impl Filesystem for ShardGhost {
    fn lookup(&mut self, _req: &Request, parent: u64, _name: &OsStr, reply: ReplyEntry) {
        if parent == 1 { reply.entry(&TTL, &FILE_ATTR, 0); } 
        else { reply.error(libc::ENOENT); }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &DIR_ATTR),
            _ => reply.attr(&TTL, &FILE_ATTR),
        }
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
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        reply.attr(&TTL, &FILE_ATTR);
    }

    fn create(&mut self, _req: &Request, _parent: u64, _name: &OsStr, _mode: u32, _umask: u32, _flags: i32, reply: fuser::ReplyCreate) {
        reply.created(&TTL, &FILE_ATTR, 0, 0, 0);
    }

    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: ReplyOpen) {
        reply.opened(0, 0);
    }

    fn write(&mut self, _req: &Request, _ino: u64, _fh: u64, _offset: i64, data: &[u8], _write_flags: u32, _flags: i32, _lock_owner: Option<u64>, reply: ReplyWrite) {
        let buffer = self.buffer.clone();
        let wal = self.wal.clone();
        
        // 1. Capture the length BEFORE moving anything
        let data_len = data.len(); 
        let data_vec = data.to_vec();

        // 2. Zero-Latency Buffer Write
        // We use the original slice here to avoid any ownership issues
        if buffer.write(data).is_ok() {
            
            // 3. Fire-and-Forget Persistence
            // Move the cloned data_vec into the background
            self.rt_handle.spawn(async move {
                let _ = wal.append(&data_vec).await;
            });

            // 4. Immediate Acknowledge
            // Use the captured length variable
            reply.written(data_len as u32);
        } else {
            reply.error(libc::ENOSPC);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino == 1 && offset == 0 {
            let _ = reply.add(1, 0, FileType::Directory, ".");
            let _ = reply.add(1, 1, FileType::Directory, "..");
        }
        reply.ok();
    }
}