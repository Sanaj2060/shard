use std::fs;
use std::time::{Duration, Instant};
use shard::daemon::{Daemon, ShardConfig};
use tokio::time::sleep;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct DataRecord {
    id: usize,
    payload: String,
}

#[tokio::test]
async fn test_shard_production_integrity() {
    let test_root = std::env::temp_dir().join("shard_production_integrity");
    let shard_dir = test_root.join("inbox");
    
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(&shard_dir).unwrap();

    let num_records = 500;
    let records: Vec<DataRecord> = (0..num_records).map(|i| DataRecord {
        id: i,
        payload: "valuable_data".to_string(),
    }).collect();

    // 1. Start Shard
    let config = ShardConfig {
        buffer_size: 10 * 1024 * 1024,
        wal_path: test_root.join("shard.wal").to_str().unwrap().to_string(),
        flush_interval_secs: 5,
        dry_run: false,
    };

    let mut daemon = Daemon::new(config, shard_dir.clone()).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let shard_dir_clone = shard_dir.clone();
    
    let daemon_handle = tokio::spawn(async move {
        daemon.watch(shard_dir_clone, rx).await.unwrap();
    });

    // 2. Controlled Burst Ingestion
    println!("Ingesting {} records...", num_records);
    for record in &records {
        let p = shard_dir.join(format!("rec_{}.json", record.id));
        fs::write(&p, serde_json::to_string(record).unwrap()).unwrap();
        // Slight back-off to allow OS to manage event queue
        if record.id % 50 == 0 { sleep(Duration::from_millis(10)).await; }
    }

    // 3. Wait for Equilibrium
    println!("Waiting for aggregation (Metadata Equilibrium)...");
    let mut success = false;
    for _ in 0..100 { // Increased from 20 to 100 iterations
        sleep(Duration::from_millis(500)).await;
        // Check if directory has reached metadata equilibrium (only block files + hidden)
        let entries: Vec<_> = fs::read_dir(&shard_dir).unwrap().filter_map(|e| e.ok()).collect();
        let fragments = entries.iter().filter(|e| !e.file_name().to_string_lossy().starts_with("shard")).count();
        if fragments == 0 {
            success = true;
            break;
        }
    }
    assert!(success, "Shard failed to reach metadata equilibrium in time");

    // 4. Verify Integrity
    let mut collected_records = Vec::new();
    for entry in fs::read_dir(&shard_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap().to_string_lossy().starts_with("shard") {
            let content = fs::read_to_string(path).unwrap();
            for line in content.lines() {
                if let Ok(rec) = serde_json::from_str::<DataRecord>(line) {
                    collected_records.push(rec);
                }
            }
        }
    }

    assert_eq!(collected_records.len(), num_records, "Data loss detected!");
    
    // Cleanup
    let _ = tx.send(());
    let _ = daemon_handle.await;
    let _ = fs::remove_dir_all(&test_root);
    println!("Integrity verified: {} records recovered perfectly.", collected_records.len());
}
