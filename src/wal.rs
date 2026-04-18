use std::fs::OpenOptions;
use std::path::Path;
use anyhow::Result;
use crc32fast::Hasher;

pub struct Wal {
    #[cfg(target_os = "linux")]
    ring: rio::Rio,
    file: std::fs::File,
}

impl Wal {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new().read(true).append(true).create(true).open(path)?;
        #[cfg(target_os = "linux")]
        { Ok(Self { ring: rio::new()?, file }) }
        #[cfg(not(target_os = "linux"))]
        { Ok(Self { file }) }
    }

    pub async fn append(&self, data: &[u8]) -> Result<()> {
        let mut hasher = Hasher::new();
        hasher.update(data);
        let checksum = hasher.finalize();
        let len = data.len() as u32;

        let mut entry = Vec::with_capacity(8 + data.len());
        entry.extend_from_slice(&len.to_le_bytes());
        entry.extend_from_slice(&checksum.to_le_bytes());
        entry.extend_from_slice(data);

        #[cfg(target_os = "linux")]
        {
            self.ring.write_at(&self.file, &entry, 0).await?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            use std::io::Write;
            let mut file = &self.file;
            file.write_all(&entry)?;
            file.sync_all()?;
        }
        Ok(())
    }
}