//! Canonical head cache.
//!
//! Provides a shared holder for the latest canonical block header (and later, the
//! associated metadata snapshot) so simulators can avoid stale header lookups.

use std::sync::Arc;

use eyre::{eyre, Result};
use futures_util::StreamExt;
use reth_primitives::SealedHeader;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration, Instant};
use tracing::{error, info, warn};

use alloy_primitives::{
    Address as AlloyAddress, Bloom, Bytes as AlloyBytes, B256, B64, U256 as AlloyU256,
};
use ethers::providers::{Middleware, Provider};
use ethers::types::{Block as EthersBlock, TxHash, H256, H64, U256 as EthersU256};
#[derive(Clone, Debug)]
pub struct HeadSnapshot {
    pub number: u64,
    pub header: SealedHeader,
}

#[derive(Debug, Default)]
struct Inner {
    latest_header: RwLock<Option<SealedHeader>>, // future: include state overlay
    latest_block_number: RwLock<Option<u64>>,
    latest_snapshot: RwLock<Option<HeadSnapshot>>,
}

/// Maintains the latest canonical block header metadata for simulations.
#[derive(Debug, Clone, Default)]
pub struct CanonicalHeadCache {
    inner: Arc<Inner>,
}

impl CanonicalHeadCache {
    /// Create a new manager instance without any cached header yet.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner::default()),
        }
    }

    /// Update the latest canonical header (called by the block subscription).
    pub async fn set_latest_header(&self, header: SealedHeader) {
        let number = header.number;
        self.set_snapshot(HeadSnapshot { number, header }).await;
    }

    /// Retrieve the most recent header snapshot if present.
    pub async fn latest_header(&self) -> Option<SealedHeader> {
        self.inner.latest_header.read().await.clone()
    }

    /// Update the latest block number without requiring a hydrated header.
    pub async fn set_latest_block_number(&self, block_number: u64) {
        let mut guard = self.inner.latest_block_number.write().await;
        *guard = Some(block_number);
    }

    /// Retrieve the most recent block number if present.
    pub async fn latest_block_number(&self) -> Option<u64> {
        *self.inner.latest_block_number.read().await
    }

    pub async fn set_snapshot(&self, snapshot: HeadSnapshot) {
        self.set_latest_block_number(snapshot.number).await;

        {
            let mut header_guard = self.inner.latest_header.write().await;
            *header_guard = Some(snapshot.header.clone());
        }

        let mut snapshot_guard = self.inner.latest_snapshot.write().await;
        *snapshot_guard = Some(snapshot);
    }

    pub async fn latest_snapshot(&self) -> Option<HeadSnapshot> {
        self.inner.latest_snapshot.read().await.clone()
    }

    /// Spawn an asynchronous task that keeps the latest canonical header updated by
    /// subscribing to the node's `newHeads` stream over IPC. The task auto-reconnects
    /// on failures and updates the cached header when the notification carries enough
    /// data to hydrate a `SealedHeader`.
    pub fn spawn_head_listener(&self, ipc_path: impl Into<String>) -> JoinHandle<()> {
        let manager = self.clone();
        let ipc_path = ipc_path.into();

        tokio::spawn(async move {
            loop {
                match Provider::connect_ipc(ipc_path.clone()).await {
                    Ok(provider) => {
                        info!("Canonical head subscription connected");
                        match provider.subscribe_blocks().await {
                            Ok(mut stream) => {
                                while let Some(head) = stream.next().await {
                                    let Some(number) = head.number else {
                                        warn!(
                                            "Received head notification without block number; ignoring"
                                        );
                                        continue;
                                    };
                                    let block_number = number.as_u64();

                                    manager.set_latest_block_number(block_number).await;

                                    if let Some(sealed) = Self::sealed_header_from_block(&head) {
                                        manager
                                            .set_snapshot(HeadSnapshot {
                                                number: block_number,
                                                header: sealed,
                                            })
                                            .await;
                                    } else {
                                        error!(
                                            missing_hash = head.hash.is_none(),
                                            block_number,
                                            "Subscription block missing required header fields; snapshot not updated"
                                        );
                                    }
                                }

                                warn!("Canonical head subscription stream ended; reconnecting...");
                            }
                            Err(err) => {
                                error!("Failed to subscribe to canonical heads: {}", err);
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed to connect to Reth IPC at {}: {}", ipc_path, err);
                    }
                }

                sleep(Duration::from_secs(1)).await;
            }
        })
    }

    /// Waits until a canonical header has been observed via the subscription or times out.
    pub async fn wait_for_latest_header(&self, timeout: Duration) -> Result<SealedHeader> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(header) = self.latest_header().await {
                return Ok(header);
            }

            if Instant::now() >= deadline {
                return Err(eyre!(
                    "Timed out waiting for canonical header from subscription"
                ));
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Waits until a block number has been observed via the subscription or times out.
    pub async fn wait_for_latest_block_number(&self, timeout: Duration) -> Result<u64> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(number) = self.latest_block_number().await {
                return Ok(number);
            }

            if Instant::now() >= deadline {
                return Err(eyre!(
                    "Timed out waiting for canonical block number from subscription"
                ));
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    fn sealed_header_from_block(block: &EthersBlock<TxHash>) -> Option<SealedHeader> {
        let hash = block.hash?;
        let number = block.number?.as_u64();

        let beneficiary = block
            .author
            .unwrap_or_else(|| ethers::types::Address::zero());
        let logs_bloom = block.logs_bloom.unwrap_or_default();
        let mix_hash = block.mix_hash.unwrap_or_else(H256::zero);
        let nonce = block.nonce.unwrap_or_else(H64::zero);

        let header = alloy_consensus::Header {
            parent_hash: h256_to_b256(block.parent_hash),
            ommers_hash: h256_to_b256(block.uncles_hash),
            beneficiary: AlloyAddress::from_slice(beneficiary.as_bytes()),
            state_root: h256_to_b256(block.state_root),
            transactions_root: h256_to_b256(block.transactions_root),
            receipts_root: h256_to_b256(block.receipts_root),
            logs_bloom: Bloom::from_slice(logs_bloom.as_bytes()),
            difficulty: u256_to_alloy(block.difficulty),
            number,
            gas_limit: block.gas_limit.as_u64(),
            gas_used: block.gas_used.as_u64(),
            timestamp: block.timestamp.as_u64(),
            extra_data: AlloyBytes::copy_from_slice(block.extra_data.as_ref()),
            mix_hash: h256_to_b256(mix_hash),
            nonce: B64::from_slice(nonce.as_bytes()),
            base_fee_per_gas: block.base_fee_per_gas.map(|v| v.as_u64()),
            withdrawals_root: block.withdrawals_root.map(h256_to_b256),
            blob_gas_used: block.blob_gas_used.map(|v| v.as_u64()),
            excess_blob_gas: block.excess_blob_gas.map(|v| v.as_u64()),
            parent_beacon_block_root: block.parent_beacon_block_root.map(h256_to_b256),
            requests_hash: None,
        };

        Some(SealedHeader::new(header, h256_to_b256(hash)))
    }
}

fn h256_to_b256(value: H256) -> B256 {
    B256::from_slice(value.as_bytes())
}

fn u256_to_alloy(value: EthersU256) -> AlloyU256 {
    let mut buf = [0u8; 32];
    value.to_big_endian(&mut buf);
    AlloyU256::from_be_bytes(buf)
}
