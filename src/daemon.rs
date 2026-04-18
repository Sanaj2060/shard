use std::path::PathBuf;
use anyhow::Result;
use crate::buffer::AtomicBuffer;
use crate::wal::Wal;
use crate::ghost::ShardGhost;
use std::sync::Arc;
use tokio::fs;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ShardConfig {
    pub buffer_size: usize,
    pub wal_path: String,
    pub flush_interval_secs: u64,
    pub dry_run: bool,
}

#[derive(Clone)]
pub struct Daemon {
    buffer: Arc<AtomicBuffer>,
    wal: Arc<Wal>,
    pub config: ShardConfig,
    backing_path: PathBuf,
}

impl Daemon {
    pub fn new(config: ShardConfig, backing_path: PathBuf) -> Result<Self> {
        let buffer = Arc::new(AtomicBuffer::new(backing_path.join(".shard.buffer"), config.buffer_size)?);
        let wal = Arc::new(Wal::new(&config.wal_path)?);
        Ok(Self { 
            buffer, 
            wal, 
            config, 
            backing_path,
        })
    }

    pub async fn mount(&self, mount_point: PathBuf) -> Result<()> {
        let ghost = ShardGhost {
            buffer: self.buffer.clone(),
            wal: self.wal.clone(),
            rt_handle: tokio::runtime::Handle::current(),
        };

        // FUSE Mount options
        let options = vec![
            fuser::MountOption::AutoUnmount, 
            fuser::MountOption::AllowOther
        ];
        
        // Spawn asynchronous periodic background flusher
        let daemon_clone = self.clone();
        let flush_interval = self.config.flush_interval_secs;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(flush_interval)).await;
                let _ = daemon_clone.flush().await;
            }
        });

        // fuser::mount2 is a blocking thread, run it on a blocking task
        println!("Shard Ghost Virtual Filesystem is intercepting at: {}", mount_point.display());
        tokio::task::spawn_blocking(move || {
            fuser::mount2(ghost, mount_point, &options).expect("Failed to mount FUSE. Ensure macFUSE/libfuse is installed.");
        }).await?;

        Ok(())
    }

    pub async fn flush(&self) -> Result<()> {
        let data = self.buffer.drain();
        if data.is_empty() { return Ok(()); }
        
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
        let final_name = self.backing_path.join(format!("shard_block_{}.json", ts));
        
        fs::write(&final_name, data).await?;
        println!("Flushed block: {}", final_name.display());
        Ok(())
    }

    pub async fn process_file(&self, path: PathBuf) -> Result<()> {
        let metadata = fs::metadata(&path).await?;
        if metadata.len() > (64 * 1024 * 1024) || metadata.len() == 0 { return Ok(()); }

        let data = fs::read(&path).await?;
        self.wal.append(&data).await?;
        
        if self.buffer.write(&data).is_ok() {
            // Logic to track processed files would go here
        } else {
            self.flush().await?;
        }
        Ok(())
    }
}