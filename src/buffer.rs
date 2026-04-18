use std::sync::atomic::{AtomicUsize, Ordering};
use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::Path;
use anyhow::{Result, bail};

pub struct AtomicBuffer {
    mmap: MmapMut,
    head: AtomicUsize,
    capacity: usize,
}

impl AtomicBuffer {
    pub fn new<P: AsRef<Path>>(path: P, size: usize) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true).write(true).create(true).open(path)?;
        file.set_len(size as u64)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        Ok(Self { mmap, head: AtomicUsize::new(0), capacity: size })
    }

    pub fn write(&self, data: &[u8]) -> Result<usize> {
        let len = data.len();
        loop {
            let current_head = self.head.load(Ordering::Acquire);
            if current_head + len > self.capacity {
                bail!("Shard buffer is full. Triggering flush.");
            }
            if self.head.compare_exchange(
                current_head, 
                current_head + len, 
                Ordering::SeqCst, 
                Ordering::Relaxed
            ).is_ok() {
                let dest = unsafe {
                    std::slice::from_raw_parts_mut(
                        self.mmap.as_ptr().add(current_head) as *mut u8,
                        len
                    )
                };
                dest.copy_from_slice(data);
                return Ok(current_head);
            }
        }
    }

    pub fn drain(&self) -> Vec<u8> {
        let head = self.head.swap(0, Ordering::SeqCst);
        if head == 0 { return vec![]; }
        self.mmap[..head].to_vec()
    }

    pub fn len(&self) -> usize {
        self.head.load(Ordering::SeqCst)
    }
}