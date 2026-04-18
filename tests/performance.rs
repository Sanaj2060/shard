use std::fs;
use std::time::{Duration, Instant};
use shard::daemon::{Daemon, ShardConfig};
use tokio::time::sleep;

#[tokio::test]
async fn test_shard_ghost_performance() {
    let test_root = std::env::temp_dir().join("shard_ghost_test");
    let mount_point = test_root.join("v_inbox");
    let backing_dir = test_root.join("p_storage");
    
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(&mount_point).unwrap();
    fs::create_dir_all(&backing_dir).unwrap();

    let num_files = 10000;
    let data = b"{\"id\": 1, \"status\": \"intercepted\"}\n";

    // 1. Setup Ghost Daemon
    let config = ShardConfig {
        buffer_size: 50 * 1024 * 1024,
        wal_path: backing_dir.join(".shard.wal").to_str().unwrap().to_string(),
        flush_interval_secs: 5, // Flush every 5 seconds for the test
    };

    let daemon = Daemon::new(config, backing_dir.clone()).unwrap();
    
    // 2. Start the Mount in a background thread
    let mount_point_clone = mount_point.clone();
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            daemon.mount(mount_point_clone).await.unwrap();
        });
    });

    // Wait for FUSE to stabilize
    sleep(Duration::from_secs(2)).await;

    // 3. Performance Run: Write to the VIRTUAL mount point
    let start_shard = Instant::now();
    for i in 0..num_files {
        let file_path = mount_point.join(format!("stream_{}.json", i));
        // This 'write' is intercepted by ShardGhost::write
        fs::write(&file_path, data).unwrap();
    }
    let duration = start_shard.elapsed();

    println!("\nGhost Interception Time: {:?}", duration);
    
    // 4. Verification
    // Wait for the SLA flush to trigger
    sleep(Duration::from_secs(6)).await;
    
    let files: Vec<_> = fs::read_dir(&backing_dir).unwrap()
        .map(|r| r.unwrap().path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "json"))
        .collect();

    assert!(!files.is_empty(), "No coalesced blocks were flushed to backing storage!");
    println!("Coalesced into {} large block(s).", files.len());
}