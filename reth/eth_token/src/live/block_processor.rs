use serde::{Deserialize, Serialize};
use tx_processor::{LivePoolBuySellSimulator, ProcessedBlock};

use crate::chain_metadata::{
    TokenDiscoveryProvider, TokenMetadataProvider, UniswapV2PoolMetadataProvider,
};
use crate::manager::{
    BlockTokenProcessor, LiveTokenRetentionPolicy, LiveTokenRetentionReport,
    ProcessedTokenUpdateRouter, TokenBlockUpdateReport, TokenRegistry,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveBlockTokenProcessor {
    block_processor: BlockTokenProcessor,
}

impl LiveBlockTokenProcessor {
    pub fn new(history_limit: usize) -> Self {
        Self::from_block_processor(BlockTokenProcessor::new(history_limit))
    }

    pub fn with_registry(registry: TokenRegistry, history_limit: usize) -> Self {
        Self::from_block_processor(BlockTokenProcessor::with_registry(registry, history_limit))
    }

    pub fn with_registry_and_update_router(
        registry: TokenRegistry,
        update_router: ProcessedTokenUpdateRouter,
    ) -> Self {
        Self::from_block_processor(BlockTokenProcessor::with_registry_and_update_router(
            registry,
            update_router,
        ))
    }

    pub fn from_block_processor(mut block_processor: BlockTokenProcessor) -> Self {
        block_processor.set_live_mode(true);
        Self { block_processor }
    }

    pub fn into_inner(self) -> BlockTokenProcessor {
        self.block_processor
    }

    pub fn block_processor(&self) -> &BlockTokenProcessor {
        &self.block_processor
    }

    pub fn registry(&self) -> &TokenRegistry {
        &self.block_processor.registry
    }

    pub fn apply_retention_policy(
        &mut self,
        policy: &LiveTokenRetentionPolicy,
        current_block: u64,
    ) -> LiveTokenRetentionReport {
        self.block_processor
            .apply_retention_policy(policy, current_block)
    }

    pub fn apply_index_retention_policy(
        &mut self,
        current_block: u64,
    ) -> Option<LiveTokenRetentionReport> {
        self.block_processor
            .apply_index_retention_policy(current_block)
    }

    pub async fn process_block_live(
        &mut self,
        block: &ProcessedBlock,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport {
        self.block_processor
            .process_block_with_live_pool_simulator(block, pool_simulator)
            .await
    }

    pub async fn process_block_live_with_metadata_provider<P>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        P: TokenMetadataProvider,
    {
        self.block_processor
            .process_block_with_metadata_provider_and_live_pool_simulator(
                block,
                metadata_provider,
                pool_simulator,
            )
            .await
    }

    pub async fn process_block_live_with_discovery_provider<P>(
        &mut self,
        block: &ProcessedBlock,
        discovery_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        P: TokenDiscoveryProvider,
    {
        self.block_processor
            .process_block_with_discovery_provider_and_live_pool_simulator(
                block,
                discovery_provider,
                pool_simulator,
            )
            .await
    }

    pub async fn process_block_live_with_token_and_pool_discovery_providers<T, V>(
        &mut self,
        block: &ProcessedBlock,
        metadata_provider: &T,
        pool_metadata_provider: &V,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> TokenBlockUpdateReport
    where
        T: TokenMetadataProvider,
        V: UniswapV2PoolMetadataProvider,
    {
        self.block_processor
            .process_block_with_token_and_pool_discovery_providers_and_live_pool_simulator(
                block,
                metadata_provider,
                pool_metadata_provider,
                pool_simulator,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;

    use alloy_primitives::{address, b256, Bytes, U256};
    use reth_chain_query::provider::{BlockHeader, TransactionData, TransactionReceipt};
    use tx_processor::tx_processor::data_models::ContractCreationEvent;
    use tx_processor::{ProcessedBlock, ProcessedBlockTransactions, ProcessedTransaction};

    use crate::chain_metadata::{TokenMetadataLookup, TokenMetadataProvider};
    use crate::erc20::ERC20TokenMetadata;
    use crate::manager::TokenRegistry;

    use super::LiveBlockTokenProcessor;

    fn metadata() -> ERC20TokenMetadata {
        ERC20TokenMetadata::new(
            "0x1111111111111111111111111111111111111111",
            "Token",
            "TKN",
            18,
            "100000000000000000000",
        )
    }

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            1,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        )
    }

    fn block_header() -> BlockHeader {
        BlockHeader {
            number: 100,
            hash: b256!("9999999999999999999999999999999999999999999999999999999999999999"),
            parent_hash: b256!("8888888888888888888888888888888888888888888888888888888888888888"),
            timestamp: 1_700,
            gas_limit: 30_000_000,
            gas_used: 21_000,
            base_fee_per_gas: Some(1),
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
            requests_hash: None,
            block_access_list_hash: None,
            slot_number: None,
        }
    }

    fn block_transaction(processed: ProcessedTransaction) -> ProcessedBlockTransactions {
        ProcessedBlockTransactions {
            metadata: TransactionData {
                hash: processed.hash,
                block_number: processed.block_number,
                block_timestamp: processed.block_timestamp,
                tx_index: processed.tx_index,
                tx_number: processed.tx_index,
                from: processed.from_address,
                to: processed.to_address,
                value: processed.value,
                input: Bytes::from(processed.input.clone()),
                gas_price: U256::ZERO,
                gas_limit: 21_000,
                nonce: processed.nonce,
                transaction_type: processed.raw_tx_type,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                access_list: Vec::new(),
                blob_versioned_hashes: Vec::new(),
                max_fee_per_blob_gas: None,
                signed_authorizations: Vec::new(),
            },
            receipt: TransactionReceipt {
                tx_hash: processed.hash,
                status: processed.status,
                gas_used: 21_000,
                logs: Vec::new(),
                cumulative_gas_used: 21_000,
                effective_gas_price: U256::ZERO,
                contract_address: processed.contract_address,
                blob_gas_used: None,
            },
            processed,
            trace: None,
            processing_error: None,
        }
    }

    #[derive(Clone)]
    struct RecordingMetadataProvider {
        metadata: ERC20TokenMetadata,
        lookups: Rc<RefCell<Vec<TokenMetadataLookup>>>,
    }

    impl TokenMetadataProvider for RecordingMetadataProvider {
        fn token_metadata<'a>(
            &'a self,
            lookup: &'a TokenMetadataLookup,
        ) -> Pin<Box<dyn Future<Output = eyre::Result<Option<ERC20TokenMetadata>>> + 'a>> {
            Box::pin(async move {
                self.lookups.borrow_mut().push(lookup.clone());
                Ok(Some(self.metadata.clone()))
            })
        }
    }

    #[test]
    fn live_block_token_processor_wraps_block_processor_in_live_mode() {
        let mut registry = TokenRegistry::new();
        registry.add_token(metadata());

        let processor = LiveBlockTokenProcessor::with_registry(registry, 100);

        assert!(processor.block_processor().is_live_mode);
        assert!(processor
            .block_processor()
            .token_index
            .live_retention_policy()
            .is_some());
        assert!(
            processor
                .registry()
                .token("0x1111111111111111111111111111111111111111")
                .unwrap()
                .is_live_mode
        );
    }

    #[tokio::test]
    async fn live_block_processor_discovers_live_created_token() {
        let provider = RecordingMetadataProvider {
            metadata: metadata(),
            lookups: Rc::new(RefCell::new(Vec::new())),
        };
        let mut processor = LiveBlockTokenProcessor::new(100);
        let mut creation_tx = tx();
        creation_tx.contract_address = Some(address!("1111111111111111111111111111111111111111"));
        creation_tx
            .contract_creation_events
            .push(ContractCreationEvent {
                contract_address: address!("1111111111111111111111111111111111111111"),
            });

        let block = ProcessedBlock {
            header: block_header(),
            transactions: vec![block_transaction(creation_tx)],
        };

        let report = processor
            .block_processor
            .process_block_with_metadata_provider_test_simulator(&block, &provider)
            .await;

        assert_eq!(report.failed_transaction_count, 0);
        assert!(
            processor
                .registry()
                .token("0x1111111111111111111111111111111111111111")
                .unwrap()
                .is_live_mode
        );
    }
}
