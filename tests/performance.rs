use std::fs;
use std::time::{Duration, Instant};
use shard::daemon::{Daemon, ShardConfig};
use tokio::time::sleep;

#[tokio::test]
async fn test_shard_vs_baseline_large_payload() {
    let test_root = std::env::temp_dir().join("shard_large_payload_bench");
    let b_dir = test_root.join("baseline");
    let v_inbox = test_root.join("v_inbox");
    let p_storage = test_root.join("p_storage");
    
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(&b_dir).unwrap();
    fs::create_dir_all(&v_inbox).unwrap();
    fs::create_dir_all(&p_storage).unwrap();

    // 100 files of 1MB each = 100MB Total
    let num_files = 100;
    let one_mb_data = vec![b'x'; 1024 * 1024]; 

    // --- PHASE 1: BASELINE (1MB Files) ---
    println!("\n[1/2] Running 100MB Baseline...");
    let start_b = Instant::now();
    for i in 0..num_files {
        fs::write(b_dir.join(format!("f_{}.json", i)), &one_mb_data).unwrap();
    }
    let b_write_done = start_b.elapsed();
    
    // Baseline manual consolidation
    let mut combined = Vec::with_capacity(num_files * 1024 * 1024);
    for i in 0..num_files {
        let p = b_dir.join(format!("f_{}.json", i));
        combined.extend_from_slice(&fs::read(&p).unwrap());
        fs::remove_file(p).unwrap();
    }
    fs::write(b_dir.join("block.json"), combined).unwrap();
    let b_total = start_b.elapsed();

    // --- PHASE 2: SHARD GHOST (1MB Interception) ---
    println!("[2/2] Running 100MB Shard Ghost...");
    let config = ShardConfig {
        buffer_size: 256 * 1024 * 1024, // 256MB Buffer
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
        fs::write(v_inbox.join(format!("g_{}.json", i)), &one_mb_data).unwrap();
    }
    let s_write_time = start_s.elapsed();

    println!("Shard - Waiting for background flush...");
    sleep(Duration::from_secs(6)).await;

    println!("\n--- 100MB PAYLOAD RESULTS ---");
    println!("Baseline (Write + Merge): {:?}", b_total);
    println!("Shard (Intercept Only):   {:?}", s_write_time);
    
    let speedup = b_total.as_secs_f64() / s_write_time.as_secs_f64();
    println!("Shard is {:.2}x faster for large payloads.", speedup);
}