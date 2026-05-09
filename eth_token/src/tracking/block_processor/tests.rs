use alloy_primitives::{address, b256, Bytes, U256};
use reth_chain_query::provider::{BlockHeader, TransactionData, TransactionReceipt};
use tx_processor::tx_processor::data_models::ERC20TransferEvent;
use tx_processor::{ProcessedBlockTransactions, ProcessedTransaction};

use crate::erc20::ERC20TokenMetadata;
use crate::network::{graph::RawTokenNetworkGraph, model::TokenNetworkId};
use crate::tracking::TrackedTokenStatus;

use super::processor::BlockTokenProcessor;
use tx_processor::ProcessedBlock;
fn empty_block() -> ProcessedBlock {
    ProcessedBlock {
        header: BlockHeader {
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
        },
        transactions: Vec::new(),
    }
}

fn token_metadata(address: &str) -> ERC20TokenMetadata {
    ERC20TokenMetadata::new(address, "Test", "TST", 6, "1000000000000")
}

fn transfer_tx() -> ProcessedTransaction {
    let token = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let mut tx = ProcessedTransaction::new(
        b256!("0000000000000000000000000000000000000000000000000000000000000001"),
        100,
        1_700,
        3,
        address!("1111111111111111111111111111111111111111"),
        None,
        U256::ZERO,
        true,
        7,
        2,
        Vec::new(),
    );
    tx.erc20_transfers.push(ERC20TransferEvent {
        token_address: token,
        from_address: address!("1111111111111111111111111111111111111111"),
        to_address: address!("2222222222222222222222222222222222222222"),
        amount: U256::from(1_500_000_u64),
        log_index: 9,
    });
    tx
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

fn block(transactions: Vec<ProcessedTransaction>) -> ProcessedBlock {
    let mut block = empty_block();
    block.transactions = transactions.into_iter().map(block_transaction).collect();
    block
}

#[test]
fn live_mode_historical_simulator_report_fails_fast() {
    let mut processor = BlockTokenProcessor::new(100);
    processor.set_live_mode(true);

    let report = processor.live_mode_historical_simulator_report(&empty_block(), "process_block");

    assert_eq!(report.processed_transaction_count, 0);
    assert_eq!(report.failed_transaction_count, 1);
    assert_eq!(processor.last_block_failure_count, 1);
    assert!(processor.latest_processed_block.is_none());
    assert!(report.transaction_errors[0]
        .message
        .contains("use LiveBlockTokenProcessor with LivePoolBuySellSimulator"));
}

#[test]
fn network_updates_persist_for_updated_token_transaction() {
    let token_address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mut processor = BlockTokenProcessor::new(100);
    processor.registry.add_token(token_metadata(token_address));

    let mut token_addresses = std::collections::BTreeSet::new();
    token_addresses.insert(token_address.to_string());
    processor.apply_network_updates_for_transaction(&transfer_tx(), token_addresses);

    let graph = processor
        .network_graphs
        .get(token_address)
        .expect("graph is stored");
    assert_eq!(graph.applied_batches, 1);
    assert_eq!(graph.address_activity.len(), 2);
    assert!(graph
        .edges
        .values()
        .any(|edge| { edge.kind == crate::network::model::NetworkEdgeKind::TokenTransfer }));
}

#[tokio::test]
async fn reverted_mined_transactions_do_not_update_tokens() {
    let token_address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mut processor = BlockTokenProcessor::new(100);
    processor.registry.add_token(token_metadata(token_address));
    processor.index_registry_token(token_address, TrackedTokenStatus::Creation, 100);

    let mut tx = transfer_tx();
    tx.status = false;

    let report = processor
        .process_block_with_test_simulator(&block(vec![tx]))
        .await;

    assert_eq!(report.transaction_count, 1);
    assert_eq!(report.processed_transaction_count, 0);
    assert_eq!(report.failed_transaction_count, 0);
    assert!(report.transaction_errors.is_empty());
    assert!(report.token_updates.is_empty());
    assert!(processor.network_graphs.get(token_address).is_none());
}

#[test]
fn token_index_eviction_removes_network_graph() {
    let first = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let second = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let mut processor = BlockTokenProcessor::new_with_token_index_limit(100, Some(1));
    processor.registry.add_token(token_metadata(first));
    processor.registry.add_token(token_metadata(second));
    processor.network_graphs.insert(
        first.to_string(),
        RawTokenNetworkGraph::new(TokenNetworkId::new(None, first)),
    );

    processor.index_registry_token(first, TrackedTokenStatus::Creation, 100);
    processor.index_registry_token(second, TrackedTokenStatus::Creation, 101);

    assert!(!processor.network_graphs.contains_key(first));
    assert!(processor.registry.token(first).is_none());
    assert!(processor.registry.token(second).is_some());
}
