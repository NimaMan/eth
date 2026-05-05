use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::Result;

use super::MarketDataEvent;

#[async_trait]
pub trait MarketDataEventSink: Send + Sync {
    async fn publish(&self, event: MarketDataEvent) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopMarketDataEventSink;

#[async_trait]
impl MarketDataEventSink for NoopMarketDataEventSink {
    async fn publish(&self, _event: MarketDataEvent) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct RecordingMarketDataEventSink {
    events: Arc<RwLock<Vec<MarketDataEvent>>>,
}

impl RecordingMarketDataEventSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<MarketDataEvent> {
        self.events
            .read()
            .expect("recording market-data sink read lock should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl MarketDataEventSink for RecordingMarketDataEventSink {
    async fn publish(&self, event: MarketDataEvent) -> Result<()> {
        self.events
            .write()
            .expect("recording market-data sink write lock should not be poisoned")
            .push(event);
        Ok(())
    }
}
