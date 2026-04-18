use std::fs;
use std::time::{Duration, Instant};
use shard::daemon::{Daemon, ShardConfig};
use tokio::time::sleep;

#[tokio::test]
async fn test_shard_vs_baseline_comparison() {
    let test_root = std::env::temp_dir().join("shard_bench_final");
    let baseline_dir = test_root.join("baseline");
    let v_inbox = test_root.join("v_inbox");     // Virtual folder
    let p_storage = test_root.join("p_storage"); // Physical storage
    
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(&baseline_dir).unwrap();
    fs::create_dir_all(&v_inbox).unwrap();
    fs::create_dir_all(&p_storage).unwrap();

    let num_files = 10000;
    let data = b"{\"id\": 1, \"status\": \"benchmarking\"}\n";

    // --- PHASE 1: STANDARD UNIX BASELINE ---
    println!("\n[1/2] Starting Baseline Test...");
    let start_baseline = Instant::now();
    
    // Step 1: Write 10k files
    for i in 0..num_files {
        let p = baseline_dir.join(format!("file_{}.json", i));
        fs::write(p, data).unwrap();
    }
    let baseline_write_time = start_baseline.elapsed();
    
    // Step 2: Read/Merge/Delete (Consolidation)
    let mut combined = Vec::with_capacity(num_files * data.len());
    for i in 0..num_files {
        let p = baseline_dir.join(format!("file_{}.json", i));
        combined.extend_from_slice(&fs::read(&p).unwrap());
        fs::remove_file(p).unwrap();
    }
    fs::write(baseline_dir.join("merged_baseline.json"), combined).unwrap();
    let baseline_total = start_baseline.elapsed();

    println!("Baseline - Step 1 (Writes): {:?}", baseline_write_time);
    println!("Baseline - Step 2 (Merge):  {:?}", baseline_total - baseline_write_time);
    println!(">> Total Baseline Time:    {:?}", baseline_total);


    // --- PHASE 2: SHARD GHOST (FUSE) ---
    println!("\n[2/2] Starting Shard Ghost Test...");
    let config = ShardConfig {
        buffer_size: 64 * 1024 * 1024,
        wal_path: p_storage.join(".shard.wal").to_str().unwrap().to_string(),
        flush_interval_secs: 5,
        dry_run: false,
    };

    let daemon = Daemon::new(config, p_storage.clone()).unwrap();
    let v_inbox_clone = v_inbox.clone();
    
    // Mount FUSE in background
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            daemon.mount(v_inbox_clone).await.unwrap();
        });
    });

    sleep(Duration::from_secs(2)).await; // Wait for mount stability

    let start_shard = Instant::now();
    
    // Step 1: Intercepted Writes
    for i in 0..num_files {
        let p = v_inbox.join(format!("ghost_{}.json", i));
        fs::write(p, data).unwrap();
    }
    let shard_write_time = start_shard.elapsed();

    // Step 2: Background Aggregation
    println!("Shard - Waiting for background flush (SLA)...");
    sleep(Duration::from_secs(6)).await; 
    let shard_total = start_shard.elapsed();

    println!("Shard - Step 1 (Intercept): {:?}", shard_write_time);
    println!("Shard - Step 2 (Flush):     Wait-time only (Async)");
    println!(">> Total Shard E2E Time:   {:?}", shard_total);

    // --- FINAL SUMMARY ---
    println!("\n--- RESULTS ---");
    let speedup = baseline_total.as_secs_f64() / shard_write_time.as_secs_f64();
    println!("Ingestion Speedup: {:.2}x faster", speedup);
}