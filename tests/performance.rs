use std::fs;
use std::time::Instant;
use shard::daemon::{Daemon, ShardConfig};
use tokio::time::sleep;

#[tokio::test]
async fn test_shard_shadow_ingest_performance() {
    let test_root = std::env::temp_dir().join("shard_shadow_bench");
    let inbox = test_root.join("inbox");
    let storage = test_root.join("storage");
    
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(&inbox).unwrap();
    fs::create_dir_all(&storage).unwrap();

    // 100MB of data (100 files of 1MB)
    let num_files = 100;
    let data = vec![b'A'; 1024 * 1024]; 

    // --- PHASE 1: SHARD SHADOW INGEST ---
    println!("\n--- Starting Shard Shadow Ingest Benchmark ---");
    let config = ShardConfig {
        buffer_size: 256 * 1024 * 1024,
        wal_path: storage.join("shard.wal").to_str().unwrap().to_string(),
        flush_interval_secs: 1, 
        dry_run: false,
    };
    
    let daemon = Daemon::new(config, storage.clone()).unwrap();
    let start_shard = Instant::now();
    
    // Ingest phase
    for i in 0..num_files {
        let p = inbox.join(format!("file_{}.bin", i));
        fs::write(&p, &data).unwrap();
        daemon.process_file(p).await.unwrap();
    }
    
    // Explicit flush/stitch
    daemon.flush().await.unwrap();
    let s_total = start_shard.elapsed();

    // --- PHASE 2: INTEGRITY VERIFICATION ---
    let block_file = fs::read_dir(&storage).unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("shard_block"))
        .expect("Block file not created");

    let merged_size = fs::metadata(block_file.path()).unwrap().len();
    assert_eq!(merged_size, (num_files * 1024 * 1024) as u64, "Data loss!");
    
    println!("Shard Full Cycle: {:?}", s_total);
    println!("Integrity verified: {}MB coalesced into 1 block.", merged_size / 1024 / 1024);

    let _ = fs::remove_dir_all(&test_root);
}
