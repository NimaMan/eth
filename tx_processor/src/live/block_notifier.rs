use eyre::{eyre, Result};
use redis::{aio::ConnectionManager, Client};
use serde_json::json;

/// Publishes block notifications over Redis Pub/Sub.
pub struct RedisBlockNotifier {
    client: Client,
    channel: String,
}

impl RedisBlockNotifier {
    pub fn new(redis_url: impl AsRef<str>, channel: impl Into<String>) -> Result<Self> {
        let client = Client::open(redis_url.as_ref())
            .map_err(|err| eyre!("failed to create redis client: {}", err))?;
        Ok(Self {
            client,
            channel: channel.into(),
        })
    }

    async fn connection(&self) -> Result<ConnectionManager> {
        ConnectionManager::new(self.client.clone())
            .await
            .map_err(|err| eyre!("failed to connect to redis: {}", err))
    }

    pub async fn notify_block_processed(&self, block_number: u64) -> Result<()> {
        let mut conn = self.connection().await?;
        let payload = serde_json::to_string(&json!({ "block_number": block_number }))
            .map_err(|err| eyre!("failed to serialize notification payload: {}", err))?;
        redis::cmd("PUBLISH")
            .arg(&self.channel)
            .arg(payload)
            .query_async::<()>(&mut conn)
            .await?;
        Ok(())
    }
}
