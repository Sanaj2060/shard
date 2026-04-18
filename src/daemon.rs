use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::buffer::AtomicBuffer;
use crate::wal::Wal;
use std::sync::Arc;
use tokio::fs;
use notify::{Watcher, RecursiveMode, Config};

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
    watch_path: PathBuf,
    processed_files: Arc<std::sync::Mutex<Vec<PathBuf>>>,
}

impl Daemon {
    pub fn new(config: ShardConfig, watch_path: PathBuf) -> Result<Self> {
        let buffer = Arc::new(AtomicBuffer::new(watch_path.join(".shard.buffer"), config.buffer_size)?);
        let wal = Arc::new(Wal::new(&config.wal_path)?);
        Ok(Self { 
            buffer, 
            wal, 
            config, 
            watch_path, 
            processed_files: Arc::new(std::sync::Mutex::new(Vec::new())) 
        })
    }

    pub async fn watch(&mut self, path: PathBuf, mut stop_rx: tokio::sync::oneshot::Receiver<()>) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(2048);

        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;

        watcher.watch(&path, RecursiveMode::NonRecursive)?;

        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                Some(event) = rx.recv() => {
                    if event.kind.is_create() || event.kind.is_modify() {
                        for p in event.paths {
                            if p.is_file() && !p.file_name().unwrap().to_string_lossy().starts_with("shard") {
                                let _ = self.process_file(p).await;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn process_file(&self, path: PathBuf) -> Result<()> {
        // 1. Wait for file stability (ensure it's not being actively written)
        let mut last_size = 0;
        for _ in 0..5 {
            if let Ok(metadata) = fs::metadata(&path).await {
                let current_size = metadata.len();
                if current_size == last_size && current_size > 0 {
                    break;
                }
                last_size = current_size;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // 2. Read and ingest
        let mut data = fs::read(&path).await?;
        if data.is_empty() { return Ok(()); }
        if !data.ends_with(b"\n") { data.push(b'\n'); }
        
        self.wal.append(&data).await?;
        
        let needs_flush = {
            if self.buffer.write(&data).is_ok() {
                self.processed_files.lock().unwrap().push(path);
                self.buffer.len() > 10 * 1024 * 1024
            } else {
                true
            }
        };

        if needs_flush {
            self.flush().await?;
        }
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
        let final_name = self.watch_path.join(format!("shard_{}.block", ts));
        
        fs::write(&final_name, data).await?;
        
        for f in files_to_delete {
            let _ = fs::remove_file(f).await;
        }
        Ok(())
    }
}
