use async_trait::async_trait;
use tokio::sync::broadcast;

use super::helpers::normalize_address;
use super::progress::LiveTokenProgress;
use super::service::LiveTokenRuntime;
use super::snapshot::LiveTokenSnapshot;
use super::LiveTokenEvent;

#[async_trait]
pub trait LiveTokenReader: Send + Sync {
    async fn progress(&self) -> LiveTokenProgress;
    async fn token_snapshots(&self) -> Vec<LiveTokenSnapshot>;
    async fn token_snapshot(&self, token_address: &str) -> Option<LiveTokenSnapshot>;
    fn subscribe(&self) -> broadcast::Receiver<LiveTokenEvent>;
}

#[async_trait]
impl LiveTokenReader for LiveTokenRuntime {
    async fn progress(&self) -> LiveTokenProgress {
        self.progress().await
    }

    async fn token_snapshots(&self) -> Vec<LiveTokenSnapshot> {
        let state = self.inner.state.read().await;
        let mut snapshots = state
            .processor
            .registry()
            .tokens
            .values()
            .map(LiveTokenSnapshot::from_token)
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| {
            left.latest_activity_block
                .cmp(&right.latest_activity_block)
                .reverse()
                .then(left.contract_address.cmp(&right.contract_address))
        });
        snapshots
    }

    async fn token_snapshot(&self, token_address: &str) -> Option<LiveTokenSnapshot> {
        let state = self.inner.state.read().await;
        state
            .processor
            .registry()
            .tokens
            .get(&normalize_address(token_address))
            .map(LiveTokenSnapshot::from_token)
    }

    fn subscribe(&self) -> broadcast::Receiver<LiveTokenEvent> {
        self.inner.event_tx.subscribe()
    }
}
