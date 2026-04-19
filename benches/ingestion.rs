use criterion::{black_box, criterion_group, criterion_main, Criterion};
use shard::storage::ShardV1;
use std::fs::{File, OpenOptions};
use std::io::Write;

fn bench_ingestion(c: &mut Criterion) {
    let num_records = 10000;
    let data = vec![b'x'; 1024];

    // Shard Storage Engine
    let shard = ShardV1::new("bench.shard", 128 * 1024 * 1024).unwrap();
    c.bench_function("shard_push_record", |b| {
        b.iter(|| shard.push_record(black_box(&data)).unwrap())
    });

    // Native Baseline
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("baseline.bin")
        .unwrap();
    c.bench_function("native_fs_write", |b| {
        b.iter(|| file.write_all(black_box(&data)).unwrap())
    });
}

criterion_group!(benches, bench_ingestion);
criterion_main!(benches);
