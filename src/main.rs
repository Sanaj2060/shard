use clap::{Parser, Subcommand};
use anyhow::Result;

mod daemon; mod stitcher; mod buffer; mod wal; mod reporter;

#[derive(Parser)]
#[command(name = "shard", about = "Unix-native write-aggregator")]
struct Cli { #[command(subcommand)] command: Commands }

#[derive(Subcommand)]
enum Commands {
    /// Start monitoring a path
    Enable { 
        path: String, 
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
        Commands::Enable { path, sla, dry_run } => {
            let path_buf = std::path::Path::new(path).canonicalize()?;
            let config = daemon::ShardConfig { 
                buffer_size: 256 * 1024 * 1024, 
                flush_interval_secs: *sla, 
                dry_run: *dry_run,
                wal_path: path_buf.join(".shard.wal").to_string_lossy().into(),
            };
            
            let daemon = daemon::Daemon::new(config, path_buf.clone())?;
            println!("Shard {} on {}", if *dry_run { "SIMULATION" } else { "ACTIVE" }, path_buf.display());
            
            // Watch/Event Loop implementation here...
        }
        Commands::Scan { path } => {
            let report = reporter::Reporter::scan(path)?;
            println!("Total Files: {}", report.total_files);
            println!("Metadata Efficiency: {:.2}%", 100.0 - report.potential_savings);
        }
    }
    Ok(())
}