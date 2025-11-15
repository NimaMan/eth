use super::{ProcessedBlockSnapshot, StateOverlaySnapshot};
use eyre::{eyre, Result};
use redis::{aio::ConnectionManager, AsyncCommands, Client};

const PROCESSED_BLOCK_PREFIX: &str = "live:processed_block_snapshot:";
const LATEST_BLOCK_NUMBER_KEY: &str = "live:block_number:latest";
const STATE_OVERLAY_PREFIX: &str = "live:state_overlay_snapshot:";
const LATEST_STATE_OVERLAY_KEY: &str = "live:state_overlay_block_number:max";

/// Builder for [`LiveChainCache`].
pub struct LiveChainCacheBuilder {
    redis_url: String,
    overlay_ttl_secs: Option<usize>,
    retention: usize,
}

impl LiveChainCacheBuilder {
    pub fn new(redis_url: impl Into<String>) -> Self {
        Self {
            redis_url: redis_url.into(),
            overlay_ttl_secs: None,
            retention: 5,
        }
    }

    pub fn overlay_ttl_secs(mut self, ttl: Option<usize>) -> Self {
        self.overlay_ttl_secs = ttl;
        self
    }

    pub fn retention(mut self, retention: usize) -> Self {
        self.retention = retention;
        self
    }

    pub fn build(self) -> Result<LiveChainCache> {
        let client = Client::open(self.redis_url)?;
        Ok(LiveChainCache {
            client,
            overlay_ttl_secs: self.overlay_ttl_secs,
            retention: self.retention,
        })
    }
}

pub struct LiveChainCache {
    client: Client,
    overlay_ttl_secs: Option<usize>,
    retention: usize,
}

impl LiveChainCache {
    pub fn new(redis_url: &str) -> Result<Self> {
        LiveChainCacheBuilder::new(redis_url).build()
    }

    pub fn builder(redis_url: impl Into<String>) -> LiveChainCacheBuilder {
        LiveChainCacheBuilder::new(redis_url)
    }

    pub async fn latest_block_number(&self) -> Result<Option<u64>> {
        let mut conn = self.connection().await?;
        let value: Option<String> = conn.get(LATEST_BLOCK_NUMBER_KEY).await?;
        Ok(parse_u64(value))
    }

    pub async fn latest_state_overlay_block_number(&self) -> Result<Option<u64>> {
        let mut conn = self.connection().await?;
        let value: Option<String> = conn.get(LATEST_STATE_OVERLAY_KEY).await?;
        Ok(parse_u64(value))
    }

    pub async fn fetch_processed_block_snapshot(
        &self,
        block_number: u64,
    ) -> Result<Option<ProcessedBlockSnapshot>> {
        let mut conn = self.connection().await?;
        let key = processed_block_key(block_number);
        let payload: Option<String> = conn.get(key).await?;
        if let Some(raw) = payload {
            let snapshot: ProcessedBlockSnapshot = serde_json::from_str(&raw)
                .map_err(|err| eyre!("failed to decode processed block snapshot: {}", err))?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    /// Fetch a single processed transaction from a stored block snapshot by the
    /// transaction hash. Returns the JSON value if found.
    pub async fn find_processed_transaction(
        &self,
        block_number: u64,
        tx_hash: &str,
    ) -> Result<Option<serde_json::Value>> {
        let Some(snapshot) = self.fetch_processed_block_snapshot(block_number).await? else {
            return Ok(None);
        };
        for value in snapshot.transactions() {
            if value
                .get("hash")
                .and_then(serde_json::Value::as_str)
                .map(|hash| hash.eq_ignore_ascii_case(tx_hash))
                .unwrap_or(false)
            {
                return Ok(Some(value.clone()));
            }
        }
        Ok(None)
    }

    pub async fn fetch_state_overlay_snapshot(
        &self,
        block_number: u64,
    ) -> Result<Option<StateOverlaySnapshot>> {
        let mut conn = self.connection().await?;
        let key = state_overlay_key(block_number);
        let payload: Option<Vec<u8>> = conn.get(key).await?;
        if let Some(raw) = payload {
            let snapshot: StateOverlaySnapshot = bincode::deserialize(&raw)
                .map_err(|err| eyre!("failed to decode state overlay snapshot: {}", err))?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    pub async fn store_state_overlay_snapshot(
        &self,
        snapshot: &StateOverlaySnapshot,
    ) -> Result<()> {
        let mut conn = self.connection().await?;
        let serialized = bincode::serialize(snapshot)
            .map_err(|err| eyre!("failed to serialize state overlay snapshot: {}", err))?;
        let key = state_overlay_key(snapshot.block_number);

        let mut pipe = redis::pipe();
        pipe.cmd("SET").arg(&key).arg(&serialized);
        if let Some(ttl) = self.overlay_ttl_secs {
            pipe.cmd("EXPIRE").arg(&key).arg(ttl);
        }
        pipe.cmd("SET")
            .arg(LATEST_STATE_OVERLAY_KEY)
            .arg(snapshot.block_number);

        if self.retention > 0 && snapshot.block_number >= self.retention as u64 {
            let evict_target = snapshot.block_number.saturating_sub(self.retention as u64);
            pipe.cmd("DEL").arg(state_overlay_key(evict_target));
        }

        pipe.query_async::<_, ()>(&mut conn).await?;
        Ok(())
    }

    async fn connection(&self) -> Result<ConnectionManager> {
        let conn = self
            .client
            .get_connection_manager()
            .await
            .map_err(|err| eyre!("failed to connect to redis: {}", err))?;
        Ok(conn)
    }
}

fn parse_u64(value: Option<String>) -> Option<u64> {
    value.and_then(|v| v.parse::<u64>().ok())
}

fn processed_block_key(block_number: u64) -> String {
    format!("{}{}", PROCESSED_BLOCK_PREFIX, block_number)
}

fn state_overlay_key(block_number: u64) -> String {
    format!("{}{}", STATE_OVERLAY_PREFIX, block_number)
}
