use std::ffi::OsStr;
use std::collections::HashMap;
use fuser::{FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyEntry, ReplyDirectory, Request};
use libc::ENOENT;
use std::time::{Duration, UNIX_EPOCH};
use crate::buffer::AtomicBuffer;
use crate::wal::Wal;
use std::sync::Arc;

pub struct ShardGhost {
    pub buffer: Arc<AtomicBuffer>,
    pub wal: Arc<Wal>,
    pub rt_handle: tokio::runtime::Handle,
}

impl Filesystem for ShardGhost {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 && name == "shard_virtual.json" {
            let attr = FileAttr {
                ino: 2,
                size: self.buffer.len() as u64,
                blocks: 0,
                atime: UNIX_EPOCH.into(),
                mtime: UNIX_EPOCH.into(),
                ctime: UNIX_EPOCH.into(),
                crtime: UNIX_EPOCH.into(),
                kind: FileType::RegularFile,
                perm: 0o644,
                nlink: 1,
                uid: 501,
                gid: 20,
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.entry(&Duration::from_secs(1), &attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if ino == 1 {
            let attr = FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: UNIX_EPOCH.into(),
                mtime: UNIX_EPOCH.into(),
                ctime: UNIX_EPOCH.into(),
                crtime: UNIX_EPOCH.into(),
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 501,
                gid: 20,
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.attr(&Duration::from_secs(1), &attr);
        } else if ino == 2 {
            let attr = FileAttr {
                ino: 2,
                size: self.buffer.len() as u64,
                blocks: 0,
                atime: UNIX_EPOCH.into(),
                mtime: UNIX_EPOCH.into(),
                ctime: UNIX_EPOCH.into(),
                crtime: UNIX_EPOCH.into(),
                kind: FileType::RegularFile,
                perm: 0o644,
                nlink: 1,
                uid: 501,
                gid: 20,
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.attr(&Duration::from_secs(1), &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, _flags: i32, _lock: Option<u64>, reply: ReplyData) {
        if ino == 2 {
            let data = self.buffer.drain();
            let start = offset as usize;
            let end = std::cmp::min(start + size as usize, data.len());
            if start < data.len() {
                reply.data(&data[start..end]);
            } else {
                reply.data(&[]);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let mut entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "shard_virtual.json"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}
