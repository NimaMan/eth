use alloy_primitives::{Address, Bytes, B256, U256};
use async_trait::async_trait;
use eth_live_feed::{
    BlockTokenProcessor, BlockTokenUpdate, LiveFeedBlockInput, LiveFeedEvent, LiveFeedPipeline,
    RecordingLiveFeedEventSink,
};
use eth_live_state::{
    ChainStateSnapshotStats, EncodedChainStateSnapshot, InMemoryLiveStateStore, LiveStateReader,
    TokenLatestBlock, TokenSnapshot,
};
use reth_chain_query::provider::{BlockHeader, TransactionData, TransactionReceipt};
use tx_processor::{ProcessedBlock, ProcessedBlockTransactions, ProcessedTransaction};

#[derive(Clone, Debug)]
struct StaticBlockTokenProcessor {
    update: BlockTokenUpdate,
}

#[async_trait]
impl BlockTokenProcessor for StaticBlockTokenProcessor {
    async fn process_block_tokens(
        &self,
        _block: &ProcessedBlock,
    ) -> eth_live_feed::Result<BlockTokenUpdate> {
        Ok(self.update.clone())
    }
}

fn address(value: &str) -> Address {
    value.parse().expect("valid address")
}

fn hash(value: &str) -> B256 {
    value.parse().expect("valid hash")
}

fn block_header(block_hash: B256, parent_hash: B256) -> BlockHeader {
    BlockHeader {
        number: 512,
        hash: block_hash,
        parent_hash,
        timestamp: 1_715_100_000,
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

fn processed_tx(tx_hash: B256, token_address: Address) -> ProcessedTransaction {
    ProcessedTransaction::new(
        tx_hash,
        512,
        1_715_100_000,
        7,
        token_address,
        None,
        U256::ZERO,
        true,
        0,
        2,
        Vec::new(),
    )
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

#[tokio::test]
async fn pipeline_writes_live_state_and_emits_block_event() {
    let store = InMemoryLiveStateStore::new();
    let sink = RecordingLiveFeedEventSink::new();
    let block_hash = hash("0x0000000000000000000000000000000000000000000000000000000000000200");
    let parent_hash = hash("0x00000000000000000000000000000000000000000000000000000000000001ff");
    let token_address = address("0x00000000000000000000000000000000000000aa");

    let tx_hash = hash("0x0000000000000000000000000000000000000000000000000000000000000abc");
    let block = ProcessedBlock {
        header: block_header(block_hash, parent_hash),
        transactions: vec![block_transaction(processed_tx(tx_hash, token_address))],
    };
    let chain_state = EncodedChainStateSnapshot::new_reth_revm_cache_bincode_v1(
        510,
        512,
        block_hash,
        parent_hash,
        vec![9, 9, 9],
        Some(ChainStateSnapshotStats {
            account_count: Some(2),
            contract_count: Some(1),
            log_count: Some(0),
        }),
    );
    let token = TokenSnapshot::new(
        token_address,
        TokenLatestBlock::new(512, Some(1_715_100_000), Some(block_hash)),
    );
    let block_token_processor = StaticBlockTokenProcessor {
        update: BlockTokenUpdate {
            updated_tokens: vec![token],
            removed_tokens: Vec::new(),
        },
    };

    let pipeline = LiveFeedPipeline::new(block_token_processor, store.clone(), sink.clone());
    let event = pipeline
        .process_block(LiveFeedBlockInput::new(block, Some(chain_state)))
        .await
        .unwrap();

    assert_eq!(event.block_number, 512);
    assert_eq!(event.updated_tokens, vec![token_address]);
    assert!(event.chain_state_available);
    assert_eq!(store.latest_block_number().await.unwrap(), Some(512));
    assert_eq!(
        store
            .read_chain_state_snapshot(512)
            .await
            .unwrap()
            .unwrap()
            .payload_len(),
        3
    );
    assert!(store.read_token(token_address).await.unwrap().is_some());

    assert_eq!(sink.events(), vec![LiveFeedEvent::BlockProcessed(event)]);
}
