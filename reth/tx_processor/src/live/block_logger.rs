use crate::live::live_block_processor::LiveProcessedBlock;
use chrono::Utc;
use eyre::{eyre, Result};
use std::{
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Logs processed block statistics to a timestamped line-based log file.
pub struct BlockProcessingLogger {
    file: Mutex<std::fs::File>,
    path: PathBuf,
}

impl BlockProcessingLogger {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|err| eyre!("failed to open log file {}: {}", path.display(), err))?;

        Ok(Self {
            file: Mutex::new(file),
            path,
        })
    }

    pub fn log_block(&self, processed: &LiveProcessedBlock) -> Result<()> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let tx_count = processed.processed_block.transactions.len();
        let processing_failures = processed
            .processed_block
            .transactions
            .iter()
            .filter(|tx| tx.processing_error.is_some())
            .count();
        let duration_secs = (processed.processed_at - processed.head_arrival)
            .num_microseconds()
            .unwrap_or(0) as f64
            / 1_000_000f64;
        let line = format!(
            "{} - INFO - {}->{}|{} in {:.2}s\n",
            timestamp,
            processed.execution_info.block_number,
            tx_count,
            processing_failures,
            duration_secs
        );

        let mut guard = self
            .file
            .lock()
            .map_err(|err| eyre!("log file lock poisoned: {}", err))?;
        guard
            .write_all(line.as_bytes())
            .map_err(|err| eyre!("failed to write log {}: {}", self.path.display(), err))?;
        print!("{}", line);
        Ok(())
    }
}
