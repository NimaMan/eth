use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    error::Result,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PortfolioId, StrategyName, TokenPoolId, WalletId},
    market::PoolProtocol,
    order::{OrderIntent, OrderSide},
    position::{Position, PositionKey, PositionState},
};
use eth_live_trading::{
    derive_min_output_from_expected_output, KartalBribeRequest, KartalSubmitDirectRawResult,
    LiveDirectRawTransactionRequest, LiveTraderTxSignal, PreSubmitSimulation, TxSubmissionPolicy,
};
use serde_json::{json, Value};

use crate::{
    execution::real::{LiveTxPlanner, LiveTxSubmissionResult, LiveTxSubmitter, TxExecutorAdapter},
    EngineExecutionAdapter,
};

#[derive(Clone)]
struct FixedPlanner {
    signal: LiveTraderTxSignal,
}

#[async_trait]
impl LiveTxPlanner for FixedPlanner {
    async fn prepare_signal(&self, _intent: &OrderIntent) -> Result<LiveTraderTxSignal> {
        Ok(self.signal.clone())
    }
}

#[derive(Clone)]
struct FixedSubmitter {
    result: LiveTxSubmissionResult,
}

#[async_trait]
impl LiveTxSubmitter for FixedSubmitter {
    async fn submit_signal(
        &self,
        _signal: &LiveTraderTxSignal,
    ) -> std::result::Result<LiveTxSubmissionResult, String> {
        Ok(self.result.clone())
    }
}

fn intent(side: OrderSide) -> OrderIntent {
    let token = Address::repeat_byte(0x11);
    OrderIntent {
        trade_id: None,
        portfolio_id: PortfolioId("gate3".to_string()),
        wallet_id: WalletId("gate3-wallet".to_string()),
        strategy_name: StrategyName("gate3-alpha11".to_string()),
        side,
        token_address: token,
        pool_address: TokenPoolId::new(token, Address::repeat_byte(0x22).to_string()),
        protocol: PoolProtocol::UniswapV2,
        amount: Amount {
            raw: U256::from(1_000_000u64),
            decimals: 18,
        },
        route: None,
        max_slippage_bps: 500,
        deadline_secs: 30,
        decision_reason: None,
    }
}

fn signal() -> LiveTraderTxSignal {
    let token = Address::repeat_byte(0x11);
    LiveTraderTxSignal {
        strategy_name: "gate3-alpha11".to_string(),
        strategy_run_id: Some("gate3-run".to_string()),
        trade_id: None,
        token_address: Some(token),
        pool_address: Some(TokenPoolId::new(
            token,
            Address::repeat_byte(0x22).to_string(),
        )),
        observed_block: Some(25_128_246),
        submission_policy: TxSubmissionPolicy::PublicMempool,
        request: LiveDirectRawTransactionRequest {
            attempt_id: Some("gate3-attempt-1".to_string()),
            chain_id: 1,
            from: "0x0000000000000000000000000000000000000001".to_string(),
            to: "0x0000000000000000000000000000000000000002".to_string(),
            value: "0".to_string(),
            data: "0x".to_string(),
            gas_limit: "500000".to_string(),
            max_fee_per_gas: "1000000000".to_string(),
            max_priority_fee_per_gas: "100000000".to_string(),
            nonce: None,
            bribe: Some(KartalBribeRequest {
                priority_fee_per_gas: "100000000".to_string(),
                max_fee_per_gas: Some("1000000000".to_string()),
            }),
            simulation: None,
            metadata: json!({ "wire_protocol": "eth_direct_raw_v1" }),
        },
    }
}

fn submit_result(status: &str, tx_hash: Option<&str>) -> KartalSubmitDirectRawResult {
    KartalSubmitDirectRawResult {
        attempt_id: "gate3-attempt-1".to_string(),
        status: status.to_string(),
        tx_hash: tx_hash.map(str::to_string),
        from: "0x0000000000000000000000000000000000000001".to_string(),
        to: "0x0000000000000000000000000000000000000002".to_string(),
        nonce: Value::Null,
        gas_limit: json!("500000"),
        max_fee_per_gas: json!("1000000000"),
        max_priority_fee_per_gas: json!("100000000"),
        error: None,
        elapsed_ms: json!(1),
    }
}

async fn execute_status(status: &str, tx_hash: Option<&str>) -> ExecutionReport {
    let adapter = TxExecutorAdapter::new(
        FixedPlanner { signal: signal() },
        FixedSubmitter {
            result: submit_result(status, tx_hash).into(),
        },
    );
    adapter.execute(intent(OrderSide::Buy)).await.unwrap()
}

#[tokio::test]
async fn gate3_a2_executor_acceptance_statuses_never_confirm_without_receipt() {
    let tx_hash = "0x1111111111111111111111111111111111111111111111111111111111111111";
    let cases = [
        ("broadcast", Some(tx_hash), ExecutionStatus::Submitted),
        ("received", None, ExecutionStatus::Pending),
        ("signed", None, ExecutionStatus::Pending),
        ("dry_run", None, ExecutionStatus::Cancelled),
        ("broadcast_error", None, ExecutionStatus::Failed),
        ("rejected", None, ExecutionStatus::Cancelled),
        ("unexpected", None, ExecutionStatus::Failed),
    ];

    for (status, tx_hash, expected) in cases {
        let report = execute_status(status, tx_hash).await;

        assert_eq!(report.status, expected, "status {status}");
        assert_ne!(report.status, ExecutionStatus::Confirmed, "status {status}");
        assert!(report.filled_amount.is_none(), "status {status}");
        assert!(report.token_amount.is_none(), "status {status}");
    }
}

#[tokio::test]
async fn gate3_a2_dry_run_is_evidence_only_not_submitted_or_confirmed() {
    let report = execute_status("dry_run", None).await;

    assert_eq!(report.status, ExecutionStatus::Cancelled);
    assert!(report.tx_hash.is_none());
    assert_eq!(
        report.error.as_deref(),
        Some("tx executor dry-run; transaction was not broadcast")
    );
}

#[tokio::test]
async fn gate3_a6_submission_report_records_selected_fee_policy() {
    let tx_hash = "0x1111111111111111111111111111111111111111111111111111111111111111";
    let report = execute_status("broadcast", Some(tx_hash)).await;
    let evidence = report
        .mined_evidence
        .expect("submitted report should carry selected tx policy");

    assert_eq!(evidence.submitted_block_number, Some(25_128_246));
    assert_eq!(evidence.selected_gas_limit.as_deref(), Some("500000"));
    assert_eq!(
        evidence.selected_max_fee_per_gas_wei.as_deref(),
        Some("1000000000")
    );
    assert_eq!(
        evidence.selected_max_priority_fee_per_gas_wei.as_deref(),
        Some("100000000")
    );
    assert_eq!(
        evidence.selected_bribe_priority_fee_per_gas_wei.as_deref(),
        Some("100000000")
    );
    assert_eq!(
        evidence.selected_bribe_max_fee_per_gas_wei.as_deref(),
        Some("1000000000")
    );
}

#[test]
fn gate3_a8_min_output_and_reverting_simulation_fail_safely() {
    let expected_output = U256::from(10_000_000_000_000_000u128);
    let min_output = derive_min_output_from_expected_output(expected_output, 500).unwrap();
    assert_eq!(min_output, U256::from(9_500_000_000_000_000u128));

    let reverting = PreSubmitSimulation {
        block_number: 25_128_246,
        block_hash: Some("0xabc".to_string()),
        state_root: Some("0xdef".to_string()),
        expected_output_token: Some(Address::repeat_byte(0x11).to_string()),
        expected_output_amount: Some(expected_output.to_string()),
        min_output_amount: Some(min_output.to_string()),
        expected_recovery_eth: DecimalAmount::from(1),
        gas_used: Some(150_000),
        would_revert: true,
        metadata: json!({ "gate": "gate3-a8" }),
    };

    let error = reverting
        .validate()
        .expect_err("reverting simulation must reject");
    assert!(error.to_string().contains("would revert"));
    assert!(derive_min_output_from_expected_output(expected_output, 10_000).is_err());
}

#[test]
fn gate3_a9_buy_submitted_or_pending_is_not_sellable() {
    let token = Address::repeat_byte(0x11);
    let key = PositionKey {
        portfolio_id: PortfolioId("gate3".to_string()),
        wallet_id: WalletId("gate3-wallet".to_string()),
        strategy_name: StrategyName("gate3-alpha11".to_string()),
        token_address: token,
        pool_address: TokenPoolId::new(token, Address::repeat_byte(0x22).to_string()),
        protocol: PoolProtocol::UniswapV2,
    };
    let mut position = Position::new("gate3-position".into(), key);
    let entry_order = OrderId("entry-order".to_string());

    position.mark_intent_created(OrderSide::Buy).unwrap();
    position
        .mark_order_submitted(entry_order.clone(), OrderSide::Buy)
        .unwrap();
    assert_eq!(position.state, PositionState::BuySubmitted);
    assert!(!position.can_submit_exit());
    assert!(position.mark_intent_created(OrderSide::Sell).is_err());

    position
        .apply_execution_report(&ExecutionReport {
            order_id: entry_order.clone(),
            status: ExecutionStatus::Pending,
            tx_hash: None,
            block_number: Some(25_128_246),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            gas_cost: None,
            mined_evidence: None,
            error: None,
        })
        .unwrap();
    assert_eq!(position.state, PositionState::BuySubmitted);
    assert!(!position.can_submit_exit());
    assert!(position.mark_intent_created(OrderSide::Sell).is_err());

    position
        .apply_execution_report(&ExecutionReport {
            order_id: entry_order,
            status: ExecutionStatus::Cancelled,
            tx_hash: None,
            block_number: Some(25_128_246),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            gas_cost: None,
            mined_evidence: None,
            error: Some("tx executor dry-run; transaction was not broadcast".to_string()),
        })
        .unwrap();
    assert_eq!(position.state, PositionState::BuyCancelled);
    assert!(!position.can_submit_exit());
}
