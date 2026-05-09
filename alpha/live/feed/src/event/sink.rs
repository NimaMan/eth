use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::Result;

use super::LiveFeedEvent;

#[async_trait]
pub trait LiveFeedEventSink: Send + Sync {
    async fn publish(&self, event: LiveFeedEvent) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopLiveFeedEventSink;

#[async_trait]
impl LiveFeedEventSink for NoopLiveFeedEventSink {
    async fn publish(&self, _event: LiveFeedEvent) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct RecordingLiveFeedEventSink {
    events: Arc<RwLock<Vec<LiveFeedEvent>>>,
}

impl RecordingLiveFeedEventSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<LiveFeedEvent> {
        self.events
            .read()
            .expect("recording live-feed sink read lock should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl LiveFeedEventSink for RecordingLiveFeedEventSink {
    async fn publish(&self, event: LiveFeedEvent) -> Result<()> {
        self.events
            .write()
            .expect("recording live-feed sink write lock should not be poisoned")
            .push(event);
        Ok(())
    }
}
