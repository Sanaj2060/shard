use std::path::PathBuf;
use anyhow::Result;
use crate::buffer::AtomicBuffer;
use crate::wal::Wal;
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
    processed_files: Arc<std::sync::Mutex<Vec<PathBuf>>>,
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
            processed_files: Arc::new(std::sync::Mutex::new(Vec::new())) 
        })
    }

    #[cfg(feature = "fuse")]
    pub async fn mount(&self, mount_point: PathBuf) -> Result<()> {
        let ghost = crate::ghost::ShardGhost {
            buffer: self.buffer.clone(),
            wal: self.wal.clone(),
            rt_handle: tokio::runtime::Handle::current(),
        };

        let options = vec![fuser::MountOption::AutoUnmount, fuser::MountOption::AllowOther];
        
        let daemon_clone = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(daemon_clone.config.flush_interval_secs)).await;
                let _ = daemon_clone.flush().await;
            }
        });

        tokio::task::spawn_blocking(move || {
            fuser::mount2(ghost, mount_point, &options).expect("Failed to mount FUSE.");
        }).await?;

        Ok(())
    }

    pub async fn flush(&self) -> Result<()> {
        let (data, files_to_delete) = {
            let mut files = self.processed_files.lock().unwrap();
            let data = self.buffer.drain();
            if data.is_empty() { return Ok(()); }
            
            let files_to_delete: Vec<PathBuf> = files.drain(..).collect();
            (data, files_to_delete)
        };
        
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
        let final_name = self.backing_path.join(format!("shard_block_{}.json", ts));
        
        fs::write(&final_name, data).await?;
        
        for f in files_to_delete {
            let _ = fs::remove_file(f).await;
        }
        Ok(())
    }
}
