use alloy_primitives::{Address, B256};
use eth_live_state::{
    BlockMeta, BlockReadyNotification, EncodedChainStateSnapshot, InMemoryLiveStateStore,
    LiveStateReader, LiveStateWriter, ProcessedBlockSnapshot, ProcessedTransactionSnapshot,
    SnapshotWriteOptions, TokenLatestBlock, TokenSnapshot,
};
use serde_json::json;

fn address(value: &str) -> Address {
    value.parse().expect("valid address")
}

fn hash(value: &str) -> B256 {
    value.parse().expect("valid hash")
}

#[tokio::test]
async fn memory_store_round_trips_block_token_and_chain_state() {
    let store = InMemoryLiveStateStore::new();
    let block_hash = hash("0x00000000000000000000000000000000000000000000000000000000000000aa");
    let parent_hash = hash("0x00000000000000000000000000000000000000000000000000000000000000a9");
    let tx_hash = hash("0x00000000000000000000000000000000000000000000000000000000000000bb");
    let token_address = address("0x0000000000000000000000000000000000000011");

    let meta = BlockMeta::new(1, 100, block_hash, parent_hash, 1_715_000_000);
    let block = ProcessedBlockSnapshot::new(
        meta,
        json!({"number": "0x64", "hash": format!("{block_hash:#x}")}),
        vec![ProcessedTransactionSnapshot::new(
            tx_hash,
            0,
            json!({"hash": format!("{tx_hash:#x}"), "tx_index": 0}),
        )],
        vec![token_address],
    );

    store.write_block(block.clone()).await.unwrap();
    store
        .mark_block_ready(BlockReadyNotification::from_block(&block, 1))
        .await
        .unwrap();

    let token = TokenSnapshot::new(
        token_address,
        TokenLatestBlock::new(100, None, Some(block_hash)),
    );
    store
        .write_token(token.clone(), SnapshotWriteOptions::default())
        .await
        .unwrap();

    let chain_state = EncodedChainStateSnapshot::new_reth_revm_cache_bincode_v1(
        95,
        100,
        block_hash,
        parent_hash,
        vec![1, 2, 3],
        None,
    );
    store
        .write_chain_state_snapshot(chain_state.clone())
        .await
        .unwrap();

    assert_eq!(store.latest_block_number().await.unwrap(), Some(100));
    assert_eq!(store.latest_block_hash().await.unwrap(), Some(block_hash));
    assert_eq!(
        store.latest_chain_state_block_number().await.unwrap(),
        Some(100)
    );
    assert_eq!(
        store
            .read_block_meta(100)
            .await
            .unwrap()
            .unwrap()
            .block_hash,
        block_hash
    );
    assert_eq!(
        store
            .read_processed_transaction(100, tx_hash)
            .await
            .unwrap()
            .unwrap()
            .tx_index,
        0
    );
    assert_eq!(store.read_token(token_address).await.unwrap(), Some(token));
    assert_eq!(
        store
            .read_chain_state_snapshot(100)
            .await
            .unwrap()
            .unwrap()
            .payload_len(),
        3
    );
    assert_eq!(
        store.list_token_addresses().await.unwrap(),
        vec![token_address]
    );
    assert_eq!(store.block_ready_notifications().unwrap().len(), 1);
}
