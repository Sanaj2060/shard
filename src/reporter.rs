use std::path::Path;
use anyhow::Result;
use walkdir::WalkDir;

pub struct FragmentationReport {
    pub total_files: usize,
    pub small_files: usize,
    pub total_size: u64,
    pub potential_savings: f64,
}

pub struct Reporter;

impl Reporter {
    pub fn scan<P: AsRef<Path>>(path: P) -> Result<FragmentationReport> {
        let mut total_files = 0;
        let mut small_files = 0;
        let mut total_size = 0;

        // A "small file" threshold (e.g., 64MB)
        let threshold = 64 * 1024 * 1024;

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file()) {
            
            let metadata = entry.metadata()?;
            let size = metadata.len();
            
            total_files += 1;
            total_size += size;
            
            if size < threshold {
                small_files += 1;
            }
        }

        let potential_savings = if total_files > 0 {
            (small_files as f64 / total_files as f64) * 100.0
        } else {
            0.0
        };

        Ok(FragmentationReport {
            total_files,
            small_files,
            total_size,
            potential_savings,
        })
    }
}
