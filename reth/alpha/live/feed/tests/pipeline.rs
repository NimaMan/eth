use alloy_primitives::{Address, B256};
use async_trait::async_trait;
use eth_live_feed::{
    BlockTokenProcessor, BlockTokenUpdate, LiveFeedBlockInput, LiveFeedEvent, LiveFeedPipeline,
    RecordingLiveFeedEventSink,
};
use eth_live_state::{
    BlockMeta, ChainStateSnapshotStats, EncodedChainStateSnapshot, InMemoryLiveStateStore,
    LiveStateReader, ProcessedBlockSnapshot, ProcessedTransactionSnapshot, TokenLatestBlock,
    TokenSnapshot,
};
use serde_json::json;

#[derive(Clone, Debug)]
struct StaticBlockTokenProcessor {
    update: BlockTokenUpdate,
}

#[async_trait]
impl BlockTokenProcessor for StaticBlockTokenProcessor {
    async fn process_block_tokens(
        &self,
        _block: &ProcessedBlockSnapshot,
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

#[tokio::test]
async fn pipeline_writes_live_state_and_emits_block_event() {
    let store = InMemoryLiveStateStore::new();
    let sink = RecordingLiveFeedEventSink::new();
    let block_hash = hash("0x0000000000000000000000000000000000000000000000000000000000000200");
    let parent_hash = hash("0x00000000000000000000000000000000000000000000000000000000000001ff");
    let token_address = address("0x00000000000000000000000000000000000000aa");

    let block = ProcessedBlockSnapshot::new(
        BlockMeta::new(1, 512, block_hash, parent_hash, 1_715_100_000),
        json!({"number": "0x200"}),
        vec![ProcessedTransactionSnapshot::new(
            hash("0x0000000000000000000000000000000000000000000000000000000000000abc"),
            7,
            json!({"hash": "0xabc", "tx_index": 7}),
        )],
        vec![token_address],
    );
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
