use eyre::{eyre, Result};
use redis::aio::ConnectionManager;
use redis::streams::{StreamId, StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, Client, FromRedisValue, Value};
use serde::{Deserialize, Serialize};

// Redis XREAD BLOCK 0 sleeps until the live block processor publishes the next
// stream event. The event is emitted after the processed block is written.
const BLOCK_UNTIL_EVENT_MS: usize = 0;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RedisBlockStreamEvent {
    pub stream_id: String,
    pub block_number: u64,
}

#[derive(Clone)]
pub struct RedisBlockStream {
    client: Client,
    stream_key: String,
    latest_block_key: String,
}

impl RedisBlockStream {
    pub fn new(
        redis_url: impl AsRef<str>,
        stream_key: impl Into<String>,
        latest_block_key: impl Into<String>,
    ) -> Result<Self> {
        let client = Client::open(redis_url.as_ref())
            .map_err(|err| eyre!("failed to create redis client: {}", err))?;
        Ok(Self {
            client,
            stream_key: stream_key.into(),
            latest_block_key: latest_block_key.into(),
        })
    }

    async fn connection(&self) -> Result<ConnectionManager> {
        ConnectionManager::new(self.client.clone())
            .await
            .map_err(|err| eyre!("failed to connect to redis: {}", err))
    }

    pub async fn latest_block_number(&self) -> Result<Option<u64>> {
        let mut conn = self.connection().await?;
        let latest: Option<u64> = conn
            .get(&self.latest_block_key)
            .await
            .map_err(|err| eyre!("failed to read {}: {}", self.latest_block_key, err))?;
        Ok(latest)
    }

    pub async fn read_after(&self, last_stream_id: &str) -> Result<Vec<RedisBlockStreamEvent>> {
        let mut conn = self.connection().await?;
        let opts = StreamReadOptions::default().block(BLOCK_UNTIL_EVENT_MS);
        let reply: Option<StreamReadReply> = conn
            .xread_options(&[self.stream_key.as_str()], &[last_stream_id], &opts)
            .await
            .map_err(|err| eyre!("failed to read redis stream {}: {}", self.stream_key, err))?;
        Ok(reply
            .map(|reply| stream_events(reply, &self.stream_key))
            .transpose()?
            .unwrap_or_default())
    }
}

pub fn missing_blocks_after(last_applied_block: Option<u64>, latest_block: u64) -> Vec<u64> {
    let start = last_applied_block
        .and_then(|block| block.checked_add(1))
        .unwrap_or(latest_block);
    if start > latest_block {
        return Vec::new();
    }
    (start..=latest_block).collect()
}

fn stream_events(
    reply: StreamReadReply,
    expected_stream_key: &str,
) -> Result<Vec<RedisBlockStreamEvent>> {
    let mut events = Vec::new();
    for key in reply.keys {
        if key.key != expected_stream_key {
            continue;
        }
        for id in key.ids {
            if let Some(event) = stream_event(id)? {
                events.push(event);
            }
        }
    }
    Ok(events)
}

fn stream_event(id: StreamId) -> Result<Option<RedisBlockStreamEvent>> {
    let Some(value) = id.map.get("block_number") else {
        return Ok(None);
    };
    Ok(Some(RedisBlockStreamEvent {
        stream_id: id.id,
        block_number: redis_value_to_u64(value)?,
    }))
}

fn redis_value_to_u64(value: &Value) -> Result<u64> {
    u64::from_redis_value(value)
        .map_err(|err| eyre!("invalid redis block_number field {:?}: {}", value, err))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use redis::streams::{StreamId, StreamKey, StreamReadReply};
    use redis::Value;

    use super::{missing_blocks_after, stream_events};

    #[test]
    fn missing_blocks_are_strictly_after_last_applied() {
        assert_eq!(missing_blocks_after(Some(10), 13), vec![11, 12, 13]);
        assert!(missing_blocks_after(Some(13), 13).is_empty());
        assert_eq!(missing_blocks_after(None, 13), vec![13]);
    }

    #[test]
    fn stream_event_parses_block_number() {
        let mut map = HashMap::new();
        map.insert("block_number".to_string(), Value::Int(250));
        let reply = StreamReadReply {
            keys: vec![StreamKey {
                key: "eth/live/blocks".to_string(),
                ids: vec![StreamId {
                    id: "1-0".to_string(),
                    map,
                }],
            }],
        };

        let events = stream_events(reply, "eth/live/blocks").expect("parse stream event");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].stream_id, "1-0");
        assert_eq!(events[0].block_number, 250);
    }
}
