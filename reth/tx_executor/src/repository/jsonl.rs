use crate::{
    error::{EthTxExecutorError, Result},
    types::ExecutionStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub attempt_id: String,
    pub created_at: DateTime<Utc>,
    pub status: ExecutionStatus,
    pub message: Option<String>,
    pub payload: Value,
}

impl ExecutionEvent {
    pub fn new(
        attempt_id: impl Into<String>,
        status: ExecutionStatus,
        message: Option<String>,
        payload: Value,
    ) -> Self {
        Self {
            attempt_id: attempt_id.into(),
            created_at: Utc::now(),
            status,
            message,
            payload,
        }
    }
}

#[async_trait]
pub trait ExecutionRecorder: Send + Sync {
    async fn record(&self, event: ExecutionEvent) -> Result<()>;
}

#[derive(Default)]
pub struct NoopRecorder;

#[async_trait]
impl ExecutionRecorder for NoopRecorder {
    async fn record(&self, _event: ExecutionEvent) -> Result<()> {
        Ok(())
    }
}

pub struct JsonlRecorder {
    path: PathBuf,
    lock: Mutex<()>,
}

impl JsonlRecorder {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }
}

#[async_trait]
impl ExecutionRecorder for JsonlRecorder {
    async fn record(&self, event: ExecutionEvent) -> Result<()> {
        let _guard = self.lock.lock().await;
        let parent = self.path.parent().ok_or_else(|| {
            EthTxExecutorError::Repository(format!("journal path {:?} has no parent", self.path))
        })?;
        tokio::fs::create_dir_all(parent).await?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        let mut line = serde_json::to_vec(&event)?;
        line.push(b'\n');
        file.write_all(&line).await?;
        Ok(())
    }
}
