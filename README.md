# Shard

### The Metadata Tax is over.

We are suffocating our systems with tiny files. Every small write is a debt—a tax paid in IOPS, inode exhaustion, and cloud provider premiums. We treat the filesystem like a dumping ground, and the filesystem is fighting back.

Shard is a high-performance, Unix-native write-aggregator. It transparently coalesces fragmented writes into optimized, immutable blocks, ensuring storage efficiency and query performance.

Stop paying the tax.

## The Vision
Storage should be invisible. Shard turns high-velocity, small-file streams into optimized blocks without the overhead of a traditional database. It is the missing link in the modern data stack: a daemon that never sleeps and an engine that manages your metadata so you don't have to.

## How it Works
1. **Intercept**: Shard monitors directories using native OS events (`inotify`/`fsevent`).
2. **Buffer & Log**: Incoming writes are staged in a lock-free, memory-mapped buffer and mirrored to a crash-safe Write-Ahead Log (WAL).
3. **Aggregate**: When size thresholds or latency SLAs (via heartbeat) are met, Shard coalesces fragments into a single, optimized block file.
4. **Finalize**: Fragmented source files are atomically cleaned up, refunding your "Metadata Tax" by drastically reducing inode usage.

## Quick Start
```bash
# Enable Shard on a directory with a 10s SLA
shard enable ./data --sla 10 &

# Scan for fragmentation and potential savings
shard scan ./data

# Monitor state
shard status ./data

# Disable/Cleanup
shard disable ./data
```

## Performance & Reliability
- **Metadata Equilibrium**: Reduces inode count from thousands to one, solving filesystem bloat.
- **Durable**: Every write is mirrored to a crash-consistent WAL.
- **Non-Blocking**: Designed to be invisible to your application, acting as a concierge for your data.

License: MIT
