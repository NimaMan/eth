use async_trait::async_trait;
use eth_alpha_core::{
    ids::{OrderId, PositionId, TradeId},
    order::OrderSide,
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use super::*;

fn submitted(side: OrderSide, token: Address) -> SubmittedExecutionRecord {
    SubmittedExecutionRecord {
        order_id: OrderId("order-1".to_string()),
        tx_hash: "0x1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap(),
        submitted_block_number: Some(99),
        position_id: PositionId("pos-1".to_string()),
        trade_id: Some(TradeId("trade-1".to_string())),
        order_side: side,
        token_address: token,
        selected_gas_limit: Some("500000".to_string()),
        selected_max_fee_per_gas_wei: Some("100000000000".to_string()),
        selected_max_priority_fee_per_gas_wei: Some("50000000000".to_string()),
        selected_bribe_priority_fee_per_gas_wei: Some("40000000000".to_string()),
        selected_bribe_max_fee_per_gas_wei: Some("100000000000".to_string()),
        gas_policy_action: Some("mempool_race_exit".to_string()),
        gas_policy_signal: Some("MempoolLpApproval".to_string()),
        gas_policy_status: Some("selected".to_string()),
        gas_policy_profile: Some("p90".to_string()),
        gas_policy_profiles: Some(vec![
            "p90".to_string(),
            "p50".to_string(),
            "normal".to_string(),
        ]),
        gas_rank_source: Some("chain_server_gas_rank".to_string()),
        gas_estimated_max_cost_eth: Some("0.01".to_string()),
        gas_estimated_priority_spend_eth: Some("0.004".to_string()),
        gas_policy_guard: Some("priority_fee_budget".to_string()),
        private_execution_transport: None,
        bundle_hash: None,
        bundle_target_block: None,
        bundle_max_block: None,
        gas_policy_tail_after_tx_hash: None,
        gas_policy_dependency_priority_fee_wei: None,
        gas_policy_dependency_gas_price_wei: None,
    }
}

fn receipt(
    status: &str,
    vault: Address,
    token: Address,
    signature: &str,
    words: [U256; 3],
) -> RpcTransactionReceipt {
    receipt_at(status, vault, token, signature, words, 100, 7)
}

fn receipt_at(
    status: &str,
    vault: Address,
    token: Address,
    signature: &str,
    words: [U256; 3],
    block_number: u64,
    transaction_index: u64,
) -> RpcTransactionReceipt {
    serde_json::from_value(json!({
        "blockNumber": format!("0x{block_number:x}"),
        "blockHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "transactionIndex": format!("0x{transaction_index:x}"),
        "status": status,
        "gasUsed": "0x5208",
        "cumulativeGasUsed": "0xa410",
        "effectiveGasPrice": "0x3b9aca00",
        "gasPrice": "0x3b9aca00",
        "logs": [{
            "address": vault.to_string(),
            "topics": [
                event_signature_topic(signature).to_string(),
                indexed_address_topic(token).to_string()
            ],
            "data": format!(
                "0x{}{}{}",
                hex::encode(words[0].to_be_bytes::<32>()),
                hex::encode(words[1].to_be_bytes::<32>()),
                hex::encode(words[2].to_be_bytes::<32>())
            )
        }]
    }))
    .unwrap()
}

#[test]
fn gate3_a5_successful_buy_receipt_records_vault_event_amounts_and_gas() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let receipt = receipt(
        "0x1",
        vault,
        token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
    );

    let report = match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, None, vault)
        .unwrap()
    {
        ReceiptReconciliation::Final(report) => report,
        ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
    };

    assert_eq!(report.status, ExecutionStatus::Confirmed);
    assert_eq!(report.block_number, Some(100));
    assert_eq!(report.filled_amount.unwrap().raw, U256::from(10u64));
    assert_eq!(report.token_amount.unwrap().raw, U256::from(20u64));
    assert_eq!(report.gas_used, Some(21_000));
    assert_eq!(
        report.gas_cost.as_ref().map(|amount| amount.raw),
        Some(U256::from(21_000_000_000_000u64))
    );
}

#[test]
fn gate3_a3_receipt_evidence_records_inclusion_position_and_backtest_lag() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let receipt = receipt(
        "0x1",
        vault,
        token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
    );

    let report = match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, None, vault)
        .unwrap()
    {
        ReceiptReconciliation::Final(report) => report,
        ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
    };
    let evidence = report.mined_evidence.expect("mined evidence");

    assert_eq!(evidence.receipt_block_number, Some(100));
    assert_eq!(
        evidence.block_hash,
        Some(
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .unwrap()
        )
    );
    assert_eq!(evidence.transaction_index, Some(7));
    assert_eq!(evidence.cumulative_gas_used, Some(42_000));
    assert_eq!(evidence.submitted_block_number, Some(99));
    assert_eq!(evidence.expected_confirmation_block, Some(100));
    assert_eq!(evidence.confirmation_lag_blocks, Some(0));
}

#[test]
fn tail_entry_receipt_records_dependency_ordering() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let receipt = receipt_at(
        "0x1",
        vault,
        token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        100,
        7,
    );
    let dependency_receipt = receipt_at(
        "0x1",
        vault,
        token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        100,
        3,
    );
    let mut submitted = submitted(OrderSide::Buy, token);
    submitted.gas_policy_action = Some("tail_entry_buy".to_string());
    submitted.gas_policy_tail_after_tx_hash = Some(format!("0x{}", "33".repeat(32)));

    let report =
        match reconcile_receipt(&submitted, &receipt, Some(&dependency_receipt), vault).unwrap() {
            ReceiptReconciliation::Final(report) => report,
            ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
        };
    let evidence = report.mined_evidence.expect("mined evidence");

    assert_eq!(
        evidence.bundle_ordering_status.as_deref(),
        Some("verified_same_block_after_dependency")
    );
    assert_eq!(evidence.bundle_dependency_block_number, Some(100));
    assert_eq!(evidence.bundle_dependency_transaction_index, Some(3));
    assert_eq!(
        evidence.gas_policy_tail_after_tx_hash.as_deref(),
        submitted.gas_policy_tail_after_tx_hash.as_deref()
    );
}

#[test]
fn regular_receipt_ignores_legacy_null_tail_dependency_hash() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let receipt = receipt_at(
        "0x1",
        vault,
        token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
        100,
        7,
    );
    let mut submitted = submitted(OrderSide::Buy, token);
    submitted.gas_policy_action = Some("entry_buy".to_string());
    submitted.gas_policy_tail_after_tx_hash = Some("null".to_string());

    let report = match reconcile_receipt(&submitted, &receipt, None, vault).unwrap() {
        ReceiptReconciliation::Final(report) => report,
        ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
    };
    let evidence = report.mined_evidence.expect("mined evidence");

    assert_eq!(report.status, ExecutionStatus::Confirmed);
    assert_eq!(evidence.bundle_ordering_status, None);
    assert_eq!(evidence.gas_policy_tail_after_tx_hash, None);
}

#[test]
fn gate3_a6_receipt_evidence_records_actual_paid_gas_cost() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let receipt = receipt(
        "0x1",
        vault,
        token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
    );

    let report = match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, None, vault)
        .unwrap()
    {
        ReceiptReconciliation::Final(report) => report,
        ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
    };
    let evidence = report.mined_evidence.expect("mined evidence");

    assert_eq!(
        evidence.effective_gas_price_wei.as_deref(),
        Some("1000000000")
    );
    assert_eq!(evidence.legacy_gas_price_wei.as_deref(), Some("1000000000"));
    assert_eq!(
        evidence.paid_gas_cost_wei.as_deref(),
        Some("21000000000000")
    );
    assert_eq!(evidence.selected_gas_limit.as_deref(), Some("500000"));
    assert_eq!(
        evidence.selected_max_fee_per_gas_wei.as_deref(),
        Some("100000000000")
    );
    assert_eq!(
        evidence.selected_max_priority_fee_per_gas_wei.as_deref(),
        Some("50000000000")
    );
    assert_eq!(
        evidence.selected_bribe_priority_fee_per_gas_wei.as_deref(),
        Some("40000000000")
    );
    assert_eq!(
        evidence.selected_bribe_max_fee_per_gas_wei.as_deref(),
        Some("100000000000")
    );
    assert_eq!(
        evidence.gas_policy_action.as_deref(),
        Some("mempool_race_exit")
    );
    assert_eq!(evidence.gas_policy_profile.as_deref(), Some("p90"));
    assert_eq!(
        evidence.gas_policy_profiles.as_deref(),
        Some(["p90".to_string(), "p50".to_string(), "normal".to_string()].as_slice())
    );
    assert_eq!(
        evidence.gas_policy_guard.as_deref(),
        Some("priority_fee_budget")
    );
    assert_eq!(
        report
            .gas_cost
            .as_ref()
            .map(|amount| amount.raw.to_string()),
        evidence.paid_gas_cost_wei
    );
}

#[test]
fn gate3_a7_receipt_evidence_records_finality_policy() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let receipt = receipt(
        "0x1",
        vault,
        token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
    );

    let report = match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, None, vault)
        .unwrap()
    {
        ReceiptReconciliation::Final(report) => report,
        ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
    };
    let evidence = report.mined_evidence.expect("mined evidence");

    assert_eq!(evidence.receipt_status.as_deref(), Some("0x1"));
    assert_eq!(evidence.accepted_confirmation_depth, Some(1));
    assert_eq!(evidence.recheck_confirmation_depth, Some(3));
}

#[test]
fn gate3_a5_successful_sell_receipt_records_vault_event_amounts_and_gas() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let receipt = receipt(
        "0x1",
        vault,
        token,
        EMERGENCY_SOLD_V2_SIGNATURE,
        [U256::from(20u64), U256::from(9u64), U256::from(1u64)],
    );

    let report = match reconcile_receipt(&submitted(OrderSide::Sell, token), &receipt, None, vault)
        .unwrap()
    {
        ReceiptReconciliation::Final(report) => report,
        ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
    };

    assert_eq!(report.status, ExecutionStatus::Confirmed);
    assert_eq!(report.filled_amount.unwrap().raw, U256::from(9u64));
    assert!(report.token_amount.is_none());
    assert_eq!(
        report.gas_cost.as_ref().map(|amount| amount.raw),
        Some(U256::from(21_000_000_000_000u64))
    );
}

#[test]
fn gate3_a4_successful_receipt_without_vault_event_stays_unresolved() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let wrong_token = Address::repeat_byte(0x44);
    let receipt = receipt(
        "0x1",
        vault,
        wrong_token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
    );

    let result =
        reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, None, vault).unwrap();

    assert!(matches!(result, ReceiptReconciliation::Unresolved(_)));
}

#[test]
fn failed_receipt_marks_order_failed() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let receipt = receipt(
        "0x0",
        vault,
        token,
        BOUGHT_V2_SIGNATURE,
        [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
    );

    let report = match reconcile_receipt(&submitted(OrderSide::Buy, token), &receipt, None, vault)
        .unwrap()
    {
        ReceiptReconciliation::Final(report) => report,
        ReceiptReconciliation::Unresolved(issue) => panic!("{issue:?}"),
    };

    assert_eq!(report.status, ExecutionStatus::Failed);
    assert!(report.error.unwrap().contains("status=0x0"));
}

#[derive(Clone)]
struct CountingReceiptProvider {
    calls: Arc<AtomicUsize>,
    receipt: Option<RpcTransactionReceipt>,
}

#[async_trait]
impl ReceiptProvider for CountingReceiptProvider {
    async fn transaction_receipt(&self, _tx_hash: TxHash) -> Result<Option<RpcTransactionReceipt>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.receipt.clone())
    }
}

#[tokio::test]
async fn reconciliation_waits_until_next_processed_block() {
    let vault = Address::repeat_byte(0x22);
    let token = Address::repeat_byte(0x33);
    let calls = Arc::new(AtomicUsize::new(0));
    let reconciler = VaultReceiptReconciler::new(
        CountingReceiptProvider {
            calls: calls.clone(),
            receipt: Some(receipt(
                "0x1",
                vault,
                token,
                BOUGHT_V2_SIGNATURE,
                [U256::from(10u64), U256::from(20u64), U256::from(1u64)],
            )),
        },
        vault,
    );

    let early = reconciler
        .reconcile_after_processed_block(vec![submitted(OrderSide::Buy, token)], Some(99))
        .await
        .unwrap();

    assert!(early.reports.is_empty());
    assert!(early.unresolved.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let ready = reconciler
        .reconcile_after_processed_block(vec![submitted(OrderSide::Buy, token)], Some(100))
        .await
        .unwrap();

    assert_eq!(ready.reports.len(), 1);
    assert_eq!(ready.reports[0].status, ExecutionStatus::Confirmed);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
