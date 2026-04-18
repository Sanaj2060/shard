use std::fs;
use std::time::Instant;
use shard::daemon::{Daemon, ShardConfig};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct DataRecord { id: usize, payload: String }

#[tokio::test]
async fn test_shard_vs_baseline_performance() {
    let test_root = std::env::temp_dir().join("shard_perf_final");
    let b_dir = test_root.join("baseline");
    let s_inbox = test_root.join("inbox");
    let s_storage = test_root.join("storage");
    
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(&b_dir).unwrap();
    fs::create_dir_all(&s_inbox).unwrap();
    fs::create_dir_all(&s_storage).unwrap();

    let num_records = 1000;
    let records: Vec<DataRecord> = (0..num_records).map(|i| DataRecord {
        id: i, payload: "valuable_data".into() 
    }).collect();

    // --- 1. BASELINE (Write + Batch Merge) ---
    let start_b = Instant::now();
    for rec in &records {
        fs::write(b_dir.join(format!("rec_{}.json", rec.id)), serde_json::to_string(rec).unwrap()).unwrap();
    }
    let mut combined = String::new();
    for entry in fs::read_dir(&b_dir).unwrap() {
        let path = entry.unwrap().path();
        combined.push_str(&fs::read_to_string(&path).unwrap());
        fs::remove_file(path).unwrap();
    }
    fs::write(b_dir.join("final.block"), combined).unwrap();
    let b_total = start_b.elapsed();

    // --- 2. SHARD (Shadow Ingest) ---
    let config = ShardConfig {
        buffer_size: 256 * 1024 * 1024,
        wal_path: s_storage.join("shard.wal").to_str().unwrap().to_string(),
        flush_interval_secs: 3600,
        dry_run: false,
    };
    let daemon = Daemon::new(config, s_storage.clone()).unwrap();
    let start_s = Instant::now();
    
    for rec in &records {
        let p = s_inbox.join(format!("rec_{}.json", rec.id));
        fs::write(&p, serde_json::to_string(rec).unwrap()).unwrap();
        daemon.process_file(p).await.unwrap();
    }
    daemon.flush().await.unwrap();
    let s_total = start_s.elapsed();

    println!("\n--- Performance Comparison ({} records) ---", num_records);
    println!("Baseline (Write + Batch): {:?}", b_total);
    println!("Shard (Shadow Ingest):    {:?}", s_total);
    
    let speedup = b_total.as_secs_f64() / s_total.as_secs_f64();
    println!("Speedup Factor: {:.2}x", speedup);
    
    let _ = fs::remove_dir_all(&test_root);
}
