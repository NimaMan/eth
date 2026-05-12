use crate::live::block_snapshot::LiveBlockSnapshot;
use eyre::{eyre, Result};
use redis::{aio::ConnectionManager, Client};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tx_simulator::live_chain_data::{live_data_registry::keys, ChainStateSnapshot};

/// Writes live block snapshots into Redis using the current processed-block shape.
pub struct RedisBlockPublisher {
    client: Client,
    ttl_seconds: Option<usize>,
    max_blocks: Option<usize>,
    processed_block_stream: String,
}

impl RedisBlockPublisher {
    pub fn new(
        redis_url: impl AsRef<str>,
        ttl_seconds: Option<usize>,
        max_blocks: Option<usize>,
        processed_block_stream: impl Into<String>,
    ) -> Result<Self> {
        let client = Client::open(redis_url.as_ref())
            .map_err(|err| eyre!("failed to create redis client: {}", err))?;
        Ok(Self {
            client,
            ttl_seconds,
            max_blocks,
            processed_block_stream: processed_block_stream.into(),
        })
    }

    async fn connection(&self) -> Result<ConnectionManager> {
        ConnectionManager::new(self.client.clone())
            .await
            .map_err(|err| eyre!("failed to connect to redis: {}", err))
    }

    pub async fn publish_snapshot(
        &self,
        snapshot: &LiveBlockSnapshot,
        state_snapshot: Option<&ChainStateSnapshot>,
    ) -> Result<()> {
        let mut conn = self.connection().await?;
        let processed_at_ms = unix_time_ms();
        let meta_key = keys::block_meta_key(snapshot.block_number);
        let header_key = keys::block_header_key(snapshot.block_number);
        let tx_map_key = keys::processed_transactions_key(snapshot.block_number);
        let tx_index_key = keys::tx_index_key(snapshot.block_number);
        let addresses_key = keys::block_addresses_key(snapshot.block_number);
        let state_snapshot_key = keys::chain_state_snapshot_key(snapshot.block_number);
        let recent_key = keys::recent_blocks_key();
        let latest_block_number_key = keys::latest_block_number_key();
        let latest_block_hash_key = keys::latest_block_hash_key();
        let latest_chain_state_key = keys::latest_chain_state_block_number_key();
        let encoded_state_snapshot = state_snapshot
            .map(|snapshot| {
                bincode::serialize(snapshot)
                    .map_err(|err| eyre!("failed to serialize tracked live state: {}", err))
            })
            .transpose()?;
        let meta_json = serde_json::to_string(&json!({
            "chain": "eth",
            "chain_id": 1,
            "block_number": snapshot.block_number,
            "block_hash": snapshot.block_hash,
            "parent_hash": snapshot.parent_hash,
            "timestamp": snapshot.timestamp,
            "tx_count": snapshot.tx_entries.len(),
            "processed_at_ms": processed_at_ms,
        }))?;

        let mut pipe = redis::pipe();
        pipe.atomic();

        pipe.cmd("SET").arg(&meta_key).arg(meta_json);
        if let Some(ttl) = self.ttl_seconds {
            pipe.cmd("EXPIRE").arg(&meta_key).arg(ttl);
        }

        if let Some(header) = &snapshot.header_json {
            let cmd = pipe.cmd("SET");
            cmd.arg(&header_key).arg(header);
            if let Some(ttl) = self.ttl_seconds {
                cmd.arg("EX").arg(ttl);
            }
        } else {
            pipe.cmd("DEL").arg(&header_key);
        }

        pipe.cmd("DEL").arg(&tx_map_key);
        pipe.cmd("DEL").arg(&tx_index_key);
        pipe.cmd("DEL").arg(&addresses_key);
        if !snapshot.tx_entries.is_empty() {
            let cmd = pipe.cmd("HSET");
            cmd.arg(&tx_map_key);
            for entry in &snapshot.tx_entries {
                cmd.arg(&entry.hash).arg(&entry.payload_json);
            }
            if let Some(ttl) = self.ttl_seconds {
                pipe.cmd("EXPIRE").arg(&tx_map_key).arg(ttl);
            }

            let cmd = pipe.cmd("ZADD");
            cmd.arg(&tx_index_key);
            for entry in &snapshot.tx_entries {
                cmd.arg(entry.tx_index).arg(&entry.hash);
            }
            if let Some(ttl) = self.ttl_seconds {
                pipe.cmd("EXPIRE").arg(&tx_index_key).arg(ttl);
            }

            let mut addresses: Vec<&str> = snapshot
                .tx_entries
                .iter()
                .flat_map(|entry| entry.unique_addresses.iter().map(String::as_str))
                .collect();
            addresses.sort_unstable();
            addresses.dedup();
            if !addresses.is_empty() {
                let cmd = pipe.cmd("SADD");
                cmd.arg(&addresses_key);
                for address in addresses {
                    cmd.arg(address);
                }
                if let Some(ttl) = self.ttl_seconds {
                    pipe.cmd("EXPIRE").arg(&addresses_key).arg(ttl);
                }
            }
        }

        pipe.cmd("ZADD")
            .arg(&recent_key)
            .arg(snapshot.block_number)
            .arg(snapshot.block_number);

        pipe.cmd("SET")
            .arg(latest_block_number_key)
            .arg(snapshot.block_number);
        pipe.cmd("SET")
            .arg(latest_block_hash_key)
            .arg(&snapshot.block_hash);

        if let Some(encoded) = &encoded_state_snapshot {
            pipe.cmd("SET").arg(&state_snapshot_key).arg(encoded);
            if let Some(ttl) = self.ttl_seconds {
                pipe.cmd("EXPIRE").arg(&state_snapshot_key).arg(ttl);
            }
            pipe.cmd("SET")
                .arg(latest_chain_state_key)
                .arg(snapshot.block_number);
        } else {
            pipe.cmd("DEL").arg(&state_snapshot_key);
        }

        let cmd = pipe.cmd("XADD");
        cmd.arg(&self.processed_block_stream);
        if let Some(maxlen) = self.max_blocks.filter(|maxlen| *maxlen > 0) {
            cmd.arg("MAXLEN").arg("=").arg(maxlen);
        }
        cmd.arg("*")
            .arg("chain")
            .arg("eth")
            .arg("chain_id")
            .arg(1)
            .arg("block_number")
            .arg(snapshot.block_number)
            .arg("block_hash")
            .arg(&snapshot.block_hash)
            .arg("tx_count")
            .arg(snapshot.tx_entries.len())
            .arg("processed_at_ms")
            .arg(processed_at_ms);
        if let Some(parent_hash) = &snapshot.parent_hash {
            cmd.arg("parent_hash").arg(parent_hash);
        }

        let _: redis::Value = pipe.query_async(&mut conn).await?;
        self.prune_blocks(&mut conn, snapshot.block_number).await?;
        Ok(())
    }

    async fn prune_blocks(&self, conn: &mut ConnectionManager, block_number: u64) -> Result<()> {
        let Some(max_blocks) = self.max_blocks else {
            return Ok(());
        };
        if max_blocks == 0 {
            return Ok(());
        }

        let cutoff = block_number.saturating_sub(max_blocks.saturating_sub(1) as u64);
        if cutoff == 0 {
            return Ok(());
        }
        let recent_key = keys::recent_blocks_key();
        let max_score = cutoff.saturating_sub(1);
        let old_blocks: Vec<String> = redis::cmd("ZRANGEBYSCORE")
            .arg(recent_key)
            .arg("-inf")
            .arg(max_score)
            .query_async(conn)
            .await?;

        if old_blocks.is_empty() {
            return Ok(());
        }

        let mut pipe = redis::pipe();
        pipe.atomic();
        for block in &old_blocks {
            if let Ok(block_number) = block.parse::<u64>() {
                pipe.cmd("DEL")
                    .arg(keys::block_meta_key(block_number))
                    .arg(keys::block_header_key(block_number))
                    .arg(keys::processed_transactions_key(block_number))
                    .arg(keys::tx_index_key(block_number))
                    .arg(keys::block_addresses_key(block_number))
                    .arg(keys::chain_state_snapshot_key(block_number));
            }
        }
        pipe.cmd("ZREM").arg(recent_key).arg(old_blocks);
        pipe.query_async::<()>(&mut *conn).await?;

        Ok(())
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
