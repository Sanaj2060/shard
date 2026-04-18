# Shard

### The Metadata Tax is over.

We are suffocating our systems with tiny files. Every small write is a debt—a tax paid in IOPS, inode exhaustion, and cloud provider premiums. We treat the filesystem like a dumping ground, and the filesystem is fighting back.

Shard is a high-performance, Unix-native write-aggregator. It sits between your chaos and your storage, transparently coalescing fragmented writes into "Perfect Parquet" blocks using zero-copy stitching.

Stop paying the tax.

## The Vision
Storage should be invisible. Shard turns high-velocity, small-file streams into optimized, immutable blocks without the overhead of a traditional database. It's the missing link in the modern data stack: a buffer that remembers, a stitcher that optimizes, and a daemon that never sleeps.

## How it Works: The Atomic Buffer
Shard utilizes a lock-free circular buffer and an `O_DIRECT` Write-Ahead Log (WAL).
1. **Intercept**: Shard watches directories via `inotify` or `io_uring`.
2. **Buffer**: Incoming writes are staged in a zero-copy atomic buffer.
3. **Stitch**: When thresholds are met, Shard stitches row groups into optimized Parquet files without full decompression.
4. **Finalize**: Fragmented source files are atomically replaced by optimized Shard blocks.

## Quick Start
Install the Shard daemon and enable it on your data directory.

```bash
# Install
cargo install shard-cli

# Enable on a directory
shard enable ./data

# Check fragmentation status
shard status ./data

# Manually trigger a coalesce
shard flush ./data
```

## Performance
- **Zero-Copy**: Data moves from buffer to disk with minimal CPU intervention.
- **io_uring**: High-depth asynchronous I/O for maximum throughput.
- **Metadata-Light**: Reduces inode pressure by 100x for small-file workloads.

License: MIT
