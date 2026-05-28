use crate::tx_processor::data_models::ProcessedTransaction;
/// Type definitions for trading viability analysis
use alloy_primitives::{Address, B256, U256};
use reth_chain_query::common_addresses::{KnownV2Protocol, KnownV3Protocol};
use reth_chain_query::provider::BlockHeader;
use serde::{Deserialize, Serialize};
use tx_simulator::tx_builders::amm_swap_route::AmmSwapRoute;

pub const DEFAULT_GAS_LIMIT_NO_PRIOR: u64 = 5_000_000;
pub const DEFAULT_BUY_GAS_LIMIT: u64 = 450_000;
pub const DEFAULT_APPROVE_GAS_LIMIT: u64 = 200_000;
pub const DEFAULT_SELL_GAS_LIMIT: u64 = 450_000;

/// Parameters controlling a pool buy/sell simulation run.
#[derive(Debug, Clone)]
pub struct PoolBuySellParameters {
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub test_amount: U256,
    pub buyer_address: Address,
    pub prior_txs: Vec<ProcessedTransaction>,
    pub block_number: Option<u64>,
    pub block_header: Option<BlockHeader>,
    pub slippage_tolerance: f64,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub buy_gas_limit: u64,
    pub approve_gas_limit: u64,
    pub sell_gas_limit: u64,
    pub weth_address: Address,
    pub denom_address: Address,
    pub denom_decimals: u8,
    pub block_delay: u64,
    pub token_decimals: u8,
    pub uniswap_v4_config: Option<UniswapV4PoolConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniswapV4PoolConfig {
    pub pool_manager: Address,
    pub pool_id: B256,
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
    pub hook_data: Vec<u8>,
}

impl Default for PoolBuySellParameters {
    fn default() -> Self {
        Self {
            token_address: Address::ZERO,
            pool_address: Address::ZERO,
            pool_type: PoolType::UniswapV2,
            test_amount: U256::ZERO,
            buyer_address: Address::from([
                0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D,
                0x80, 0x4a, 0x24, 0xbd, 0x56, 0x89,
            ]),
            prior_txs: Vec::new(),
            block_number: None,
            block_header: None,
            slippage_tolerance: 5.0,
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            buy_gas_limit: DEFAULT_BUY_GAS_LIMIT,
            approve_gas_limit: DEFAULT_APPROVE_GAS_LIMIT,
            sell_gas_limit: DEFAULT_SELL_GAS_LIMIT,
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA,
                0xd9, 0x08, 0x3C, 0x75, 0x6C, 0xc2,
            ]),
            denom_address: Address::ZERO,
            denom_decimals: 0,
            block_delay: 0,
            token_decimals: 0,
            uniswap_v4_config: None,
        }
    }
}

impl PoolBuySellParameters {
    pub fn new(token_address: Address, pool_address: Address, pool_type: PoolType) -> Self {
        Self {
            token_address,
            pool_address,
            pool_type,
            ..Default::default()
        }
    }

    pub fn with_test_amount(mut self, amount: U256) -> Self {
        self.test_amount = amount;
        self
    }

    pub fn with_buyer(mut self, address: Address) -> Self {
        self.buyer_address = address;
        self
    }

    pub fn with_prior_tx(mut self, tx: ProcessedTransaction) -> Self {
        self.prior_txs = vec![tx];
        self
    }

    pub fn with_prior_transactions<I>(mut self, txs: I) -> Self
    where
        I: IntoIterator<Item = ProcessedTransaction>,
    {
        self.prior_txs = txs.into_iter().collect();
        self
    }

    pub fn with_denom_address(mut self, denom: Address) -> Self {
        self.denom_address = denom;
        self
    }

    pub fn with_denom_decimals(mut self, decimals: u8) -> Self {
        self.denom_decimals = decimals;
        self
    }

    pub fn with_block(mut self, block: u64) -> Self {
        self.block_number = Some(block);
        self
    }

    pub fn with_block_header(mut self, header: BlockHeader) -> Self {
        self.block_header = Some(header);
        self
    }

    pub fn with_block_delay(mut self, delay: u64) -> Self {
        self.block_delay = delay;
        self
    }

    pub fn with_token_decimals(mut self, decimals: u8) -> Self {
        self.token_decimals = decimals;
        self
    }

    pub fn with_legacy_gas_price(mut self, gas_price: u128) -> Self {
        self.gas_price = Some(gas_price);
        self
    }

    pub fn with_max_fee_per_gas(mut self, max_fee: u128) -> Self {
        self.max_fee_per_gas = Some(max_fee);
        self
    }

    pub fn with_max_priority_fee_per_gas(mut self, max_priority: u128) -> Self {
        self.max_priority_fee_per_gas = Some(max_priority);
        self
    }

    pub fn with_uniswap_v4_config(mut self, config: UniswapV4PoolConfig) -> Self {
        self.uniswap_v4_config = Some(config);
        self
    }
}

/// Supported DEX pool types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolType {
    UniswapV2,
    UniswapV3 { fee_tier: u32 }, // 500, 3000, 10000 (0.05%, 0.3%, 1%)
    SushiSwapV3 { fee_tier: u32 },
    PancakeSwapV3 { fee_tier: u32 },
    SushiSwap,
    PancakeSwapV2,
    ShibaSwapV2,
    FraxswapV2,
    Curve,
    Balancer,
    UniswapV4,
}

impl PoolType {
    pub fn known_v2_protocol(self) -> Option<KnownV2Protocol> {
        match self {
            Self::UniswapV2 => Some(KnownV2Protocol::UniswapV2),
            Self::SushiSwap => Some(KnownV2Protocol::SushiSwapV2),
            Self::PancakeSwapV2 => Some(KnownV2Protocol::PancakeSwapV2),
            Self::ShibaSwapV2 => Some(KnownV2Protocol::ShibaSwapV2),
            Self::FraxswapV2 => Some(KnownV2Protocol::FraxswapV2),
            _ => None,
        }
    }

    pub fn known_v3_protocol(self) -> Option<KnownV3Protocol> {
        match self {
            Self::UniswapV3 { .. } => Some(KnownV3Protocol::UniswapV3),
            Self::SushiSwapV3 { .. } => Some(KnownV3Protocol::SushiSwapV3),
            Self::PancakeSwapV3 { .. } => Some(KnownV3Protocol::PancakeSwapV3),
            _ => None,
        }
    }

    pub fn v3_fee_tier(self) -> Option<u32> {
        match self {
            Self::UniswapV3 { fee_tier }
            | Self::SushiSwapV3 { fee_tier }
            | Self::PancakeSwapV3 { fee_tier } => Some(fee_tier),
            _ => None,
        }
    }

    pub fn from_known_v3_protocol(protocol: KnownV3Protocol, fee_tier: u32) -> Self {
        match protocol {
            KnownV3Protocol::UniswapV3 => Self::UniswapV3 { fee_tier },
            KnownV3Protocol::SushiSwapV3 => Self::SushiSwapV3 { fee_tier },
            KnownV3Protocol::PancakeSwapV3 => Self::PancakeSwapV3 { fee_tier },
        }
    }

    pub fn amm_swap_route(self, pool: Address) -> Option<AmmSwapRoute> {
        match self {
            Self::UniswapV3 { fee_tier } => Some(AmmSwapRoute::UniswapV3 { pool, fee_tier }),
            Self::SushiSwapV3 { fee_tier } => Some(AmmSwapRoute::SushiswapV3 { pool, fee_tier }),
            Self::PancakeSwapV3 { fee_tier } => {
                Some(AmmSwapRoute::PancakeSwapV3 { pool, fee_tier })
            }
            _ => self
                .known_v2_protocol()
                .map(|protocol| AmmSwapRoute::V2Router {
                    pool,
                    router: protocol.router(),
                }),
        }
    }

    pub fn from_known_v2_protocol(protocol: KnownV2Protocol) -> Self {
        match protocol {
            KnownV2Protocol::UniswapV2 => Self::UniswapV2,
            KnownV2Protocol::SushiSwapV2 => Self::SushiSwap,
            KnownV2Protocol::PancakeSwapV2 => Self::PancakeSwapV2,
            KnownV2Protocol::ShibaSwapV2 => Self::ShibaSwapV2,
            KnownV2Protocol::FraxswapV2 => Self::FraxswapV2,
        }
    }
}

/// Aggregated outcome from the pool buy/sell simulation pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolBuySellSimulationResult {
    pub pool_type: PoolType,
    pub pool_address: Address,
    pub token_address: Address,
    pub can_buy: bool,
    pub can_approve: bool,
    pub can_sell: bool,
    pub is_tradeable: bool,
    pub buy_tax_percent: f64,
    pub sell_tax_percent: f64,
    pub tokens_received: U256,
    pub denom_spent: U256,
    pub denom_received: U256,
    pub buy_transaction: ProcessedTransaction,
    pub sell_transaction: ProcessedTransaction,
    pub approve_transaction: ProcessedTransaction,
    pub prior_transactions: Vec<ProcessedTransaction>,
    pub failure_reason: Option<String>,
    pub block_number: u64,
}

#[derive(Debug, Clone)]
pub struct TradingSequenceResult {
    pub setup_tx_result: Option<ProcessedTransaction>,
    pub token_buy_result: ProcessedTransaction,
    pub token_approve_result: ProcessedTransaction,
    pub token_sell_result: ProcessedTransaction,
    pub tokens_bought_amount: U256,
    pub denom_spent_on_tokens: U256,
    pub denom_received_from_selling_tokens: U256,
    pub buy_tax_percentage: f64,
    pub sell_tax_percentage: f64,
    pub can_buy: bool,
    pub can_approve: bool,
    pub can_sell: bool,
    pub all_transactions_succeeded: bool,
    pub token_is_tradeable: bool,
    pub total_gas_used: u64,
    pub simulation_block_number: u64,
    pub failure_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_type_round_trips_known_v2_protocols() {
        for protocol in KnownV2Protocol::ALL {
            let pool_type = PoolType::from_known_v2_protocol(protocol);

            assert_eq!(pool_type.known_v2_protocol(), Some(protocol));
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptionalSetupBuyApproveSellResult {
    pub setup_tx_result: Option<ProcessedTransaction>,
    pub token_buy_result: ProcessedTransaction,
    pub token_approve_result: ProcessedTransaction,
    pub token_sell_result: ProcessedTransaction,
    pub tokens_bought_amount: U256,
    pub denom_spent_on_tokens: f64,
    pub denom_received_from_selling_tokens: f64,
    pub buy_tax_percentage: f64,
    pub sell_tax_percentage: f64,
    pub can_buy: bool,
    pub can_approve: bool,
    pub can_sell: bool,
    pub all_transactions_succeeded: bool,
    pub token_is_tradeable: bool,
    pub total_gas_used: u64,
    pub simulation_block_number: u64,
    pub failure_reason: Option<String>,
}

impl TradingSequenceResult {
    pub fn to_optional_setup_result(&self) -> OptionalSetupBuyApproveSellResult {
        OptionalSetupBuyApproveSellResult {
            setup_tx_result: self.setup_tx_result.clone(),
            token_buy_result: self.token_buy_result.clone(),
            token_approve_result: self.token_approve_result.clone(),
            token_sell_result: self.token_sell_result.clone(),
            tokens_bought_amount: self.tokens_bought_amount,
            denom_spent_on_tokens: self
                .denom_spent_on_tokens
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
                / 1e18,
            denom_received_from_selling_tokens: self
                .denom_received_from_selling_tokens
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
                / 1e18,
            buy_tax_percentage: self.buy_tax_percentage,
            sell_tax_percentage: self.sell_tax_percentage,
            can_buy: self.can_buy,
            can_approve: self.can_approve,
            can_sell: self.can_sell,
            all_transactions_succeeded: self.all_transactions_succeeded,
            token_is_tradeable: self.token_is_tradeable,
            total_gas_used: self.total_gas_used,
            simulation_block_number: self.simulation_block_number,
            failure_reason: self.failure_reason.clone(),
        }
    }
}
