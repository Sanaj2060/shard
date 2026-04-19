use memmap2::{MmapMut, MmapOptions};
use std::fs::{OpenOptions, File};
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::Result;

pub struct ShardV1 {
    mmap: MmapMut,
    tail: AtomicUsize,
    capacity: usize,
}

impl ShardV1 {
    pub fn new(path: &str, capacity: usize) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        file.set_len(capacity as u64)?;
        
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        
        Ok(Self {
            mmap,
            tail: AtomicUsize::new(0),
            capacity,
        })
    }

    // High-performance append: perform memcpy into pre-allocated mmap
    // Returns the offset for future lookup
    pub fn push_record(&self, data: &[u8]) -> Result<usize> {
        let len = data.len();
        let offset = self.tail.fetch_add(len, Ordering::SeqCst);
        
        if offset + len > self.capacity {
            self.tail.fetch_sub(len, Ordering::SeqCst);
            return Err(anyhow::anyhow!("Storage capacity reached"));
        }

        // Zero-copy memory write
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.mmap.as_ptr().add(offset) as *mut u8, len);
        }
        
        Ok(offset)
    }

    // Zero-copy read: return slice of existing memory
    pub fn get_record(&self, offset: usize, len: usize) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.mmap.as_ptr().add(offset), len)
        }
    }
}
