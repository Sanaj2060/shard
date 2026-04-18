# Shard

### The Metadata Tax is over.

We are suffocating our systems with tiny files. Every small write is a debt—a tax paid in IOPS, inode exhaustion, and cloud provider premiums. We treat the filesystem like a dumping ground, and the filesystem is fighting back.

Shard is a high-performance, Unix-native write-aggregator. It transparently coalesces fragmented writes into optimized, immutable blocks, ensuring storage efficiency and query performance.

Stop paying the tax.

## The Vision
Storage should be invisible. Shard turns high-velocity, small-file streams into optimized blocks without the overhead of a traditional database. It is the missing link in the modern data stack: a daemon that never sleeps and an engine that manages your metadata so you don't have to.

## Architecture: "Shadow Ingest"
Shard uses a native, zero-copy architecture:
1. **Inbox**: Applications write natively to an "Inbox" directory.
2. **Shadow Swallow**: Shard detects file closure events, ingests data into a high-speed memory-mapped buffer, and moves/deletes the original fragment.
3. **Durable WAL**: Every write is mirrored to a crash-safe Write-Ahead Log (WAL) using `io_uring` (Linux) or standard async I/O.
4. **Aggregate**: Fragments are coalesced into `shard_*.block` files, refunding your "Metadata Tax."

## Quick Start
```bash
# Enable Shard on a directory
shard enable ./data --sla 10 &

# Scan for fragmentation metrics
shard scan ./data

# Monitor daemon status
shard status ./data

# Disable monitoring and flush final blocks
shard disable ./data
```

## Developer Guide
Shard requires native Linux features (inotify/io_uring) for optimal performance.

1. **VM Lab Setup**: Launch an Ubuntu VM (20GB disk recommended).
2. **Environment**: Install `build-essential` and `rustup`.
3. **Build**: `cargo build`
4. **Performance Testing**: Run `cargo test --test performance -- --nocapture` to verify that Shard reaches "Metadata Equilibrium" (reducing thousands of files to one) while maintaining 100% data integrity.

## Reliability
- **Metadata Equilibrium**: Reduces inode count from thousands to one, solving filesystem bloat.
- **Durable**: Every write is mirrored to a crash-consistent WAL.
- **Non-Blocking**: Designed to be invisible to your application, acting as an high-velocity data concierge.

License: MIT
