use super::*;

#[derive(Clone, Default)]
pub(super) struct CancelledSellWithValueAdapter;

#[async_trait::async_trait]
impl EngineExecutionAdapter for CancelledSellWithValueAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        Ok(match intent.side {
            OrderSide::Buy => ExecutionReport {
                order_id: OrderId("cancel-value-buy".to_string()),
                status: ExecutionStatus::Confirmed,
                tx_hash: None,
                block_number: Some(2),
                filled_amount: Some(intent.amount.clone()),
                token_amount: Some(intent.amount),
                gas_used: Some(21_000),
                gas_cost: Some(Amount {
                    raw: U256::from(21_000_000u64),
                    decimals: 18,
                }),
                error: None,
            },
            OrderSide::Sell => ExecutionReport {
                order_id: OrderId("cancel-value-sell".to_string()),
                status: ExecutionStatus::Cancelled,
                tx_hash: None,
                block_number: Some(2),
                filled_amount: None,
                token_amount: None,
                gas_used: None,
                gas_cost: None,
                error: Some("uneconomic sell".to_string()),
            },
        })
    }

    async fn simulate_position_value(
        &self,
        _position: &Position,
        pool: &PoolSnapshot,
    ) -> Result<Option<PositionValueSimulation>> {
        Ok(Some(PositionValueSimulation {
            block_number: pool.latest_block,
            current_value: Amount {
                raw: U256::from(5_000_000_000_000_000u64),
                decimals: 18,
            },
            gas_used: Some(21_000),
            error: None,
        }))
    }
}

#[derive(Clone, Default)]
pub(super) struct NoopTestExecutionAdapter;

#[async_trait::async_trait]
impl EngineExecutionAdapter for NoopTestExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        Ok(ExecutionReport {
            order_id: OrderId("noop-order".to_string()),
            status: ExecutionStatus::Failed,
            tx_hash: None,
            block_number: None,
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            gas_cost: None,
            error: Some(format!("unexpected {:?} execution in test", intent.side)),
        })
    }
}

pub(super) fn test_position(state: PositionState) -> Position {
    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let mut position = Position::new(
        PositionId("test-position".to_string()),
        PositionKey {
            portfolio_id: PortfolioId("chain-sim".to_string()),
            wallet_id: WalletId("chain-sim-wallet".to_string()),
            strategy_name: StrategyName("strategy".to_string()),
            token_address: token,
            pool_address: TokenPoolId::new(token, pool_address.to_string()),
            protocol: PoolProtocol::UniswapV2,
        },
    );
    position.state = state;
    position
}
