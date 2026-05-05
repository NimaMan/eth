//! Redis-backed cache for live chain data.
//!
//! This module provides the `LiveChainCache` struct, which is responsible for
//! interacting with a Redis instance to store and retrieve live blockchain
//! data. This includes block headers, processed transactions, and state
//! snapshots. It offers functionalities to fetch the latest block numbers,
//! processed blocks and transactions, and chain state snapshots, as well as
//! to store new state snapshots. The cache helps in bridging the gap between
//! the historical data available in the local MDBX database and the most
//! recent data from the live chain.
use crate::block_context::live_data_registry::{self, ChainStateSnapshot};
use eyre::{eyre, Result};
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde_json::Value;
use std::collections::HashMap;
use tracing::warn;

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
            retention: 10,
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
        let value: Option<String> = conn
            .get(live_data_registry::keys::latest_block_number_key())
            .await?;
        Ok(parse_u64(value))
    }

    pub async fn latest_chain_state_block_number(&self) -> Result<Option<u64>> {
        let mut conn = self.connection().await?;
        let value: Option<String> = conn
            .get(live_data_registry::keys::latest_chain_state_block_number_key())
            .await?;
        Ok(parse_u64(value))
    }

    pub async fn fetch_processed_block(&self, block_number: u64) -> Result<Option<Vec<Value>>> {
        let mut conn = self.connection().await?;
        let key = live_data_registry::keys::processed_transactions_key(block_number);
        let raw: HashMap<String, String> = conn.hgetall(&key).await?;
        if raw.is_empty() {
            return Ok(None);
        }
        let mut values = Vec::with_capacity(raw.len());
        for value in raw.values() {
            let tx: Value = serde_json::from_str(value)
                .map_err(|err| eyre!("failed to decode processed tx: {}", err))?;
            values.push(tx);
        }
        values.sort_by_key(|value| {
            value
                .get("transaction")
                .and_then(|tx| tx.get("tx_index"))
                .and_then(Value::as_u64)
                .or_else(|| value.get("tx_index").and_then(Value::as_u64))
                .unwrap_or(u64::MAX)
        });
        Ok(Some(values))
    }

    pub async fn fetch_block_header(&self, block_number: u64) -> Result<Option<String>> {
        let mut conn = self.connection().await?;
        let key = live_data_registry::keys::block_header_key(block_number);
        let payload: Option<String> = conn.get(key).await?;
        Ok(payload)
    }

    /// Return up to `limit` most recent block numbers present in the cache, descending.
    pub async fn recent_block_numbers(&self, limit: usize) -> Result<Vec<u64>> {
        let mut conn = self.connection().await?;
        if limit == 0 {
            return Ok(Vec::new());
        }

        let raw: Vec<String> = redis::cmd("ZREVRANGE")
            .arg(live_data_registry::keys::recent_blocks_key())
            .arg(0)
            .arg(limit.saturating_sub(1))
            .query_async(&mut conn)
            .await?;
        let numbers: Vec<u64> = raw
            .into_iter()
            .filter_map(|value| value.parse::<u64>().ok())
            .collect();
        if !numbers.is_empty() {
            return Ok(numbers);
        }

        let Some(latest) = self.latest_block_number().await? else {
            return Ok(Vec::new());
        };

        let mut numbers = Vec::with_capacity(limit);
        numbers.push(latest);
        let mut current = latest;
        for _ in 1..limit {
            if current == 0 {
                break;
            }
            let next = current - 1;
            let key = live_data_registry::keys::block_header_key(next);
            let exists: bool = conn.exists(&key).await?;
            if exists {
                numbers.push(next);
                current = next;
            } else {
                break;
            }
        }
        Ok(numbers)
    }

    pub async fn fetch_processed_tx(
        &self,
        block_number: u64,
        tx_hash: &str,
    ) -> Result<Option<serde_json::Value>> {
        let mut conn = self.connection().await?;
        let key = live_data_registry::keys::processed_transactions_key(block_number);
        let mut payload: Option<String> = conn.hget(&key, tx_hash).await?;
        if payload.is_none() {
            let lowered = tx_hash.to_ascii_lowercase();
            if lowered != tx_hash {
                payload = conn.hget(&key, lowered).await?;
            }
        }
        if let Some(raw) = payload {
            let tx: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|err| eyre!("failed to decode processed tx: {}", err))?;
            Ok(Some(tx))
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
        if let Some(tx) = self.fetch_processed_tx(block_number, tx_hash).await? {
            return Ok(Some(tx));
        }

        let Some(values) = self.fetch_processed_block(block_number).await? else {
            return Ok(None);
        };
        for value in values {
            if value
                .get("hash")
                .and_then(Value::as_str)
                .map(|hash| hash.eq_ignore_ascii_case(tx_hash))
                .unwrap_or(false)
            {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    pub async fn fetch_chain_state_snapshot(
        &self,
        block_number: u64,
    ) -> Result<Option<ChainStateSnapshot>> {
        let mut conn = self.connection().await?;
        let key = live_data_registry::keys::chain_state_snapshot_key(block_number);
        let payload: Option<Vec<u8>> = conn.get(key).await?;
        if let Some(raw) = payload {
            match bincode::deserialize(&raw) {
                Ok(snapshot) => Ok(Some(snapshot)),
                Err(err) => {
                    warn!(
                        block_number,
                        "ignoring undecodable live state overlay snapshot: {}", err
                    );
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    pub async fn store_chain_state_snapshot(&self, snapshot: &ChainStateSnapshot) -> Result<()> {
        let mut conn = self.connection().await?;
        let serialized = bincode::serialize(snapshot)
            .map_err(|err| eyre!("failed to serialize state overlay snapshot: {}", err))?;
        let key = live_data_registry::keys::chain_state_snapshot_key(snapshot.block_number);

        let mut pipe = redis::pipe();
        pipe.cmd("SET").arg(&key).arg(&serialized);
        if let Some(ttl) = self.overlay_ttl_secs {
            pipe.cmd("EXPIRE").arg(&key).arg(ttl);
        }
        pipe.cmd("SET")
            .arg(live_data_registry::keys::latest_chain_state_block_number_key())
            .arg(snapshot.block_number);

        if self.retention > 0 && snapshot.block_number >= self.retention as u64 {
            let evict_target = snapshot.block_number.saturating_sub(self.retention as u64);
            pipe.cmd("DEL")
                .arg(live_data_registry::keys::chain_state_snapshot_key(
                    evict_target,
                ));
        }

        pipe.query_async::<()>(&mut conn).await?;
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
