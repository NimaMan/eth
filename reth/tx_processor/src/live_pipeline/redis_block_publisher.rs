use crate::live_pipeline::block_snapshot::LiveBlockSnapshot;
use eyre::{eyre, Result};
use redis::{aio::ConnectionManager, Client};
use tx_simulator::live_chain_data::live_data_registry::keys;

/// Writes live block snapshots into Redis using the canonical schema.
pub struct RedisBlockPublisher {
    client: Client,
    ttl_seconds: Option<usize>,
}

impl RedisBlockPublisher {
    pub fn new(redis_url: impl AsRef<str>, ttl_seconds: Option<usize>) -> Result<Self> {
        let client = Client::open(redis_url.as_ref())
            .map_err(|err| eyre!("failed to create redis client: {}", err))?;
        Ok(Self {
            client,
            ttl_seconds,
        })
    }

    async fn connection(&self) -> Result<ConnectionManager> {
        ConnectionManager::new(self.client.clone())
            .await
            .map_err(|err| eyre!("failed to connect to redis: {}", err))
    }

    pub async fn publish_snapshot(&self, snapshot: &LiveBlockSnapshot) -> Result<()> {
        let mut conn = self.connection().await?;
        let header_key = keys::block_header_key(snapshot.block_number);
        let latest_key = keys::latest_block_number_key();
        let tx_map_key = keys::processed_transactions_key(snapshot.block_number);

        let mut pipe = redis::pipe();
        pipe.atomic();

        pipe.cmd("SET").arg(&latest_key).arg(snapshot.block_number);

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
        if !snapshot.tx_entries.is_empty() {
            let cmd = pipe.cmd("HSET");
            cmd.arg(&tx_map_key);
            for (hash, value) in &snapshot.tx_entries {
                cmd.arg(hash).arg(value);
            }
            if let Some(ttl) = self.ttl_seconds {
                pipe.cmd("EXPIRE").arg(&tx_map_key).arg(ttl);
            }
        }

        pipe.query_async::<_, ()>(&mut conn).await?;
        Ok(())
    }
}
