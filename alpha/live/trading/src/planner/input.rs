use eth_alpha_core::{
    ids::BlockNumber, market::PoolSnapshot, order::OrderIntent, position::Position,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::TxPrepRequestContext;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannerTxContext {
    pub tx: TxPrepRequestContext,
    pub current_block: BlockNumber,
    pub deadline_unix_secs: u64,
}

impl PlannerTxContext {
    pub fn required_state_block(&self, pool: &PoolSnapshot) -> BlockNumber {
        let mut required = self
            .tx
            .required_state_block
            .max(self.current_block)
            .max(pool.latest_block);
        if let Some(observed_block) = self.tx.observed_block {
            required = required.max(observed_block);
        }
        if let Some(creation_block) = pool.creation_block {
            required = required.max(creation_block);
        }
        required
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LivePrioritySellPlannerInput {
    pub context: PlannerTxContext,
    pub intent: OrderIntent,
    pub position: Position,
    pub pool: PoolSnapshot,
    #[serde(default)]
    pub min_output_amount: Option<String>,
    #[serde(default)]
    pub source_metadata: Value,
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use eth_alpha_core::{
        amount::DecimalAmount,
        ids::PoolAddress,
        market::{PoolProtocol, PoolSnapshot},
    };
    use serde_json::Value;

    use super::*;

    #[test]
    fn required_state_block_uses_signal_and_pool_dependencies() {
        let context = PlannerTxContext {
            tx: TxPrepRequestContext {
                chain_id: 1,
                from: Address::ZERO.to_string(),
                strategy_name: "strategy".to_string(),
                strategy_run_id: Some("run-1".to_string()),
                observed_block: Some(102),
                required_state_block: 101,
                source_metadata: Value::Null,
            },
            current_block: 100,
            deadline_unix_secs: 0,
        };
        let pool = PoolSnapshot {
            address: PoolAddress::from("0xtoken:0xpool"),
            token_address: Address::ZERO,
            protocol: PoolProtocol::UniswapV2,
            denom_address: None,
            denom_symbol: Some("WETH".to_string()),
            denom_reserve: DecimalAmount::ZERO,
            token_reserve: DecimalAmount::ZERO,
            price_denom_per_token: None,
            initial_price_denom_per_token: None,
            price_ratio_to_initial: None,
            creation_block: Some(103),
            token_decimals: Some(18),
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 104,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        };

        assert_eq!(context.required_state_block(&pool), 104);
    }
}
