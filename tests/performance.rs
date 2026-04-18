use std::fs;
use std::time::{Duration, Instant};
use shard::daemon::{Daemon, ShardConfig};
use tokio::time::sleep;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct DataRecord { id: usize, payload: String }

#[tokio::test]
async fn test_shard_vs_baseline_full_comparison() {
    let test_root = std::env::temp_dir().join("shard_perf_comparison");
    let baseline_dir = test_root.join("baseline");
    let shard_dir = test_root.join("shard_target");
    
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(&baseline_dir).unwrap();
    fs::create_dir_all(&shard_dir).unwrap();

    let num_records = 10000;
    let records: Vec<DataRecord> = (0..num_records).map(|i| DataRecord {
        id: i,
        payload: "large_payload_to_stress_the_io_subsystem_with_more_data".into(),
    }).collect();

    // --- 1. Baseline: Write + Batch Merge ---
    let start_baseline = Instant::now();
    for rec in &records {
        fs::write(baseline_dir.join(format!("rec_{}.json", rec.id)), serde_json::to_string(rec).unwrap()).unwrap();
    }
    // Simulate batch consolidation (the "manual way")
    let mut combined = String::new();
    for entry in fs::read_dir(&baseline_dir).unwrap() {
        let path = entry.unwrap().path();
        combined.push_str(&fs::read_to_string(&path).unwrap());
        combined.push('\n');
        fs::remove_file(path).unwrap();
    }
    fs::write(baseline_dir.join("final.block"), combined).unwrap();
    let baseline_duration = start_baseline.elapsed();

    // --- 2. Shard: Stream + Aggregate ---
    let config = ShardConfig {
        buffer_size: 10 * 1024 * 1024,
        wal_path: test_root.join("shard.wal").to_str().unwrap().to_string(),
        flush_interval_secs: 3600,
        dry_run: false,
    };
    let daemon = Daemon::new(config, shard_dir.clone()).unwrap();
    let start_shard = Instant::now();

    for rec in &records {
        let p = shard_dir.join(format!("rec_{}.json", rec.id));
        fs::write(&p, serde_json::to_string(rec).unwrap()).unwrap();
        daemon.process_file(p).await.unwrap();
    }
    daemon.flush().await.unwrap();
    let shard_duration = start_shard.elapsed();

    println!("\n--- Performance Comparison ({} records) ---", num_records);
    println!("Baseline (Write + Batch): {:?}", baseline_duration);
    println!("Shard (Streaming):        {:?}", shard_duration);
    
    let speedup = baseline_duration.as_secs_f64() / shard_duration.as_secs_f64();
    println!("Shard is {:.2}x {}", 
        if speedup > 1.0 { speedup } else { 1.0/speedup },
        if speedup > 1.0 { "faster" } else { "slower (due to WAL/Buffer overhead)" });

    let _ = fs::remove_dir_all(&test_root);
}
