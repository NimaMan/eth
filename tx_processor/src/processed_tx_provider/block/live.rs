use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::B256;
use eyre::{eyre, Result, WrapErr};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use reth_chain_query::provider::BlockHeader;
use serde::Deserialize;
use tx_simulator::block_context::live_data_registry::keys;

use super::compact::CompactProcessedTransaction;
use super::load::{LoadedProcessedBlock, ProcessedBlockProvider};
use super::replay_store::ProcessedBlockReplayStoreWriter;
use crate::{BlockProcessor, ProcessedBlock, ProcessedBlockSource};

#[derive(Clone)]
pub struct LiveProcessedBlockProvider {
    redis: RedisLiveProcessedBlockProvider,
    regular_provider: ProcessedBlockProvider,
}

impl LiveProcessedBlockProvider {
    pub fn new(
        redis_url: impl AsRef<str>,
        tx_processor: BlockProcessor,
        provider: Arc<reth_chain_query::RethQueryProvider>,
        replay_store_writer: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Result<Self> {
        Ok(Self {
            redis: RedisLiveProcessedBlockProvider::new(redis_url)?,
            regular_provider: ProcessedBlockProvider::new(
                tx_processor,
                provider,
                replay_store_writer,
            ),
        })
    }

    pub async fn load_block(&self, block_number: u64) -> Result<LoadedProcessedBlock> {
        match self.redis.load_block(block_number).await {
            Ok(Some(loaded)) => return Ok(loaded),
            Ok(None) => {
                tracing::warn!(
                    block_number,
                    "live Redis processed block missing after stream notification; loading from disk/Reth"
                );
            }
            Err(error) => {
                tracing::warn!(
                block_number,
                redis_error = %error,
                "live Redis processed block unavailable after stream notification; loading from disk/Reth"
                );
            }
        }

        self.regular_provider.load_block(block_number).await
    }
}

#[derive(Clone)]
struct RedisLiveProcessedBlockProvider {
    client: Client,
}

impl RedisLiveProcessedBlockProvider {
    fn new(redis_url: impl AsRef<str>) -> Result<Self> {
        let client = Client::open(redis_url.as_ref())
            .map_err(|err| eyre!("failed to create live processed block Redis client: {err}"))?;
        Ok(Self { client })
    }

    async fn connection(&self) -> Result<ConnectionManager> {
        ConnectionManager::new(self.client.clone())
            .await
            .map_err(|err| eyre!("failed to connect to live processed block Redis: {err}"))
    }

    async fn load_block(&self, block_number: u64) -> Result<Option<LoadedProcessedBlock>> {
        let started = Instant::now();
        let mut conn = self.connection().await?;

        let header_json: Option<String> = conn
            .get(keys::block_header_key(block_number))
            .await
            .wrap_err("failed to read live processed block header")?;
        let Some(header_json) = header_json else {
            return Ok(None);
        };
        let header = decode_header(&header_json)?;

        if header.number != block_number {
            return Err(eyre!(
                "live Redis header block number mismatch: requested {}, decoded {}",
                block_number,
                header.number
            ));
        }

        let payloads: HashMap<String, String> = conn
            .hgetall(keys::processed_transactions_key(block_number))
            .await
            .wrap_err("failed to read live processed transaction map")?;
        let ordered_hashes: Vec<String> = redis::cmd("ZRANGE")
            .arg(keys::tx_index_key(block_number))
            .arg(0)
            .arg(-1)
            .query_async(&mut conn)
            .await
            .wrap_err("failed to read live processed transaction index")?;

        let transactions = decode_transactions(payloads, ordered_hashes)?;
        Ok(Some(LoadedProcessedBlock {
            block: ProcessedBlock {
                header,
                transactions,
            },
            upstream_ms: started.elapsed().as_millis(),
            disk_cache_hit: false,
            disk_cache_read_ms: 0,
            disk_cache_write_ms: 0,
            source: ProcessedBlockSource::LiveRedis.as_str(),
        }))
    }
}

#[derive(Debug, Deserialize)]
struct RedisBlockHeaderPayload {
    hash: String,
    #[serde(rename = "parentHash")]
    parent_hash: String,
    number: String,
    #[serde(rename = "gasLimit")]
    gas_limit: String,
    #[serde(rename = "gasUsed")]
    gas_used: String,
    timestamp: String,
    #[serde(rename = "baseFeePerGas")]
    base_fee_per_gas: Option<String>,
    #[serde(rename = "withdrawalsRoot")]
    withdrawals_root: Option<String>,
    #[serde(rename = "blobGasUsed")]
    blob_gas_used: Option<String>,
    #[serde(rename = "excessBlobGas")]
    excess_blob_gas: Option<String>,
    #[serde(rename = "parentBeaconBlockRoot")]
    parent_beacon_block_root: Option<String>,
    #[serde(rename = "requestsHash")]
    requests_hash: Option<String>,
    #[serde(rename = "blockAccessListHash")]
    block_access_list_hash: Option<String>,
    #[serde(rename = "slotNumber")]
    slot_number: Option<String>,
}

fn decode_header(payload: &str) -> Result<BlockHeader> {
    let payload: RedisBlockHeaderPayload = serde_json::from_str(payload)
        .map_err(|err| eyre!("failed to decode live processed block header: {err}"))?;
    Ok(BlockHeader {
        number: parse_u64_string(&payload.number, "number")?,
        hash: parse_b256_string(&payload.hash, "hash")?,
        parent_hash: parse_b256_string(&payload.parent_hash, "parentHash")?,
        timestamp: parse_u64_string(&payload.timestamp, "timestamp")?,
        gas_limit: parse_u64_string(&payload.gas_limit, "gasLimit")?,
        gas_used: parse_u64_string(&payload.gas_used, "gasUsed")?,
        base_fee_per_gas: parse_optional_u64_string(
            payload.base_fee_per_gas.as_deref(),
            "baseFeePerGas",
        )?,
        withdrawals_root: parse_optional_b256_string(
            payload.withdrawals_root.as_deref(),
            "withdrawalsRoot",
        )?,
        blob_gas_used: parse_optional_u64_string(payload.blob_gas_used.as_deref(), "blobGasUsed")?,
        excess_blob_gas: parse_optional_u64_string(
            payload.excess_blob_gas.as_deref(),
            "excessBlobGas",
        )?,
        parent_beacon_block_root: parse_optional_b256_string(
            payload.parent_beacon_block_root.as_deref(),
            "parentBeaconBlockRoot",
        )?,
        requests_hash: parse_optional_b256_string(
            payload.requests_hash.as_deref(),
            "requestsHash",
        )?,
        block_access_list_hash: parse_optional_b256_string(
            payload.block_access_list_hash.as_deref(),
            "blockAccessListHash",
        )?,
        slot_number: parse_optional_u64_string(payload.slot_number.as_deref(), "slotNumber")?,
    })
}

fn decode_transactions(
    payloads: HashMap<String, String>,
    ordered_hashes: Vec<String>,
) -> Result<Vec<crate::ProcessedBlockTransactions>> {
    if payloads.is_empty() {
        if ordered_hashes.is_empty() {
            return Ok(Vec::new());
        }
        return Err(eyre!(
            "live Redis transaction index has {} hashes but transaction map is empty",
            ordered_hashes.len()
        ));
    }

    if ordered_hashes.is_empty() {
        let mut transactions = payloads
            .values()
            .map(|payload| decode_transaction(payload))
            .collect::<Result<Vec<_>>>()?;
        transactions.sort_by_key(|tx| tx.processed.tx_index);
        return Ok(transactions);
    }

    let mut transactions = Vec::with_capacity(ordered_hashes.len());
    let mut seen = HashSet::with_capacity(ordered_hashes.len());
    for hash in ordered_hashes {
        let payload = payloads
            .get(&hash)
            .or_else(|| payloads.get(&hash.to_ascii_lowercase()))
            .ok_or_else(|| eyre!("live Redis transaction index references missing hash {hash}"))?;
        transactions.push(decode_transaction(payload)?);
        seen.insert(hash.to_ascii_lowercase());
    }

    for (hash, payload) in payloads {
        if !seen.contains(&hash.to_ascii_lowercase()) {
            transactions.push(decode_transaction(&payload)?);
        }
    }
    transactions.sort_by_key(|tx| tx.processed.tx_index);
    Ok(transactions)
}

fn decode_transaction(payload: &str) -> Result<crate::ProcessedBlockTransactions> {
    let decoded: RedisProcessedTransactionPayload = serde_json::from_str(payload)
        .map_err(|err| eyre!("failed to decode compact live processed transaction JSON: {err}"))?;
    Ok(decoded
        .processed
        .into_block_transaction(decoded.processing_error))
}

#[derive(Debug, Deserialize)]
struct RedisProcessedTransactionPayload {
    #[serde(flatten)]
    processed: CompactProcessedTransaction,
    #[serde(default)]
    processing_error: Option<String>,
}

fn parse_optional_u64_string(value: Option<&str>, label: &str) -> Result<Option<u64>> {
    value
        .map(|value| parse_u64_string(value, label))
        .transpose()
}

fn parse_u64_string(value: &str, label: &str) -> Result<u64> {
    if let Some(hex) = value.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|err| eyre!("invalid {label} hex {value}: {err}"))
    } else {
        value
            .parse::<u64>()
            .map_err(|err| eyre!("invalid {label} value {value}: {err}"))
    }
}

fn parse_optional_b256_string(value: Option<&str>, label: &str) -> Result<Option<B256>> {
    value
        .map(|value| parse_b256_string(value, label))
        .transpose()
}

fn parse_b256_string(value: &str, label: &str) -> Result<B256> {
    B256::from_str(value).map_err(|err| eyre!("invalid {label} value {value}: {err}"))
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, B256, U256};
    use serde_json::json;

    use super::*;
    use crate::{CompactProcessedTransaction, ProcessedTransaction};

    #[test]
    fn decodes_header_payload() {
        let header = decode_header(
            r#"{
                "hash":"0x1111111111111111111111111111111111111111111111111111111111111111",
                "parentHash":"0x2222222222222222222222222222222222222222222222222222222222222222",
                "number":"0x2a",
                "gasLimit":"0x1c9c380",
                "gasUsed":"0x5208",
                "timestamp":"0x65a0bc00",
                "baseFeePerGas":"0x7"
            }"#,
        )
        .expect("decode header");

        assert_eq!(header.number, 42);
        assert_eq!(header.gas_limit, 30_000_000);
        assert_eq!(header.gas_used, 21_000);
        assert_eq!(header.base_fee_per_gas, Some(7));
    }

    #[test]
    fn decodes_live_transaction_payload_with_hex_input() {
        let mut tx = ProcessedTransaction::new(
            B256::repeat_byte(0x11),
            42,
            1_700_000_000,
            7,
            Address::repeat_byte(0x22),
            Some(Address::repeat_byte(0x33)),
            U256::from(123),
            true,
            9,
            2,
            vec![0xde, 0xad, 0xbe, 0xef],
        );
        tx.fees.gas_limit = 123_456;
        let mut payload =
            serde_json::to_value(CompactProcessedTransaction::from_processed(&tx)).unwrap();
        let object = payload.as_object_mut().unwrap();
        object.insert("input".to_string(), json!("0xdeadbeef"));
        object.insert("processing_error".to_string(), json!("simulated failure"));
        object
            .get_mut("fees")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("protocol_type");

        let decoded = decode_transaction(&payload.to_string()).expect("decode tx");

        assert_eq!(decoded.processed.input, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(decoded.processed.fees.gas_limit, 123_456);
        assert_eq!(
            decoded.processing_error.as_deref(),
            Some("simulated failure")
        );
    }

    #[test]
    fn rejects_legacy_dense_live_transaction_payload() {
        let tx = ProcessedTransaction::new(
            B256::repeat_byte(0x11),
            42,
            1_700_000_000,
            7,
            Address::repeat_byte(0x22),
            Some(Address::repeat_byte(0x33)),
            U256::from(123),
            true,
            9,
            2,
            vec![0xde, 0xad, 0xbe, 0xef],
        );
        let payload = serde_json::to_string(&tx).expect("legacy dense payload");

        let _ = decode_transaction(&payload).expect_err("legacy dense payload rejected");
    }
}
