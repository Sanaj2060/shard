use clap::{Parser, Subcommand};
use anyhow::Result;

mod daemon; mod stitcher; mod wal; mod reporter; mod storage; mod buffer;
#[cfg(feature = "fuse")]
mod ghost;

#[derive(Parser)]
#[command(name = "shard", about = "Unix-native FUSE write-aggregator")]
struct Cli { #[command(subcommand)] command: Commands }

#[derive(Subcommand)]
enum Commands {
    /// Mount Shard as a virtual filesystem interceptor
    Mount { 
        /// The virtual directory to mount the FUSE filesystem
        mount_point: String,
        /// The physical directory to store the flushed blocks and WAL
        backing_dir: String,
        /// Seconds before forcing a buffer flush
        #[arg(short, long, default_value_t = 30)] sla: u64, 
        #[arg(long)] dry_run: bool 
    },
    /// Scan for fragmentation metrics
    Scan { path: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Mount { mount_point, backing_dir, sla, dry_run } => {
            let mount_buf = std::path::Path::new(mount_point).canonicalize().unwrap_or_else(|_| std::path::PathBuf::from(mount_point));
            let backing_buf = std::path::Path::new(backing_dir).canonicalize().unwrap_or_else(|_| std::path::PathBuf::from(backing_dir));
            
            std::fs::create_dir_all(&mount_buf)?;
            std::fs::create_dir_all(&backing_buf)?;

            let config = daemon::ShardConfig { 
                buffer_size: 256 * 1024 * 1024, 
                flush_interval_secs: *sla, 
                dry_run: *dry_run,
                wal_path: backing_buf.join(".shard.wal").to_string_lossy().into(),
            };
            
            let daemon = daemon::Daemon::new(config, backing_buf.clone())?;
            println!("Starting Shard. Backing blocks will write to: {}", backing_buf.display());
            
            // This will block and keep the process alive while FUSE intercepts
            #[cfg(feature = "fuse")]
            daemon.mount(mount_buf).await?;
        }
        Commands::Scan { path } => {
            let report = reporter::Reporter::scan(path)?;
            println!("Total Files: {}", report.total_files);
            println!("Metadata Efficiency: {:.2}%", 100.0 - report.potential_savings);
        }
    }
    Ok(())
}