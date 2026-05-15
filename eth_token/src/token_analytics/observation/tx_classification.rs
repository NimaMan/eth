use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationTransactionType {
    Swap,
    TokenApproval,
    TokenTransfer,
    DenomTransfer,
    LpTransfer,
    LpApproval,
    PoolMint,
    PoolBurn,
    PoolSync,
    LiquidityUpdate,
    PriceUpdate,
    TradingSimulation,
    TaxSimulation,
    Bribe,
    ControlAddressActivity,
    NetworkActivity,
    Other,
}

impl ObservationTransactionType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Swap => "Pool swap",
            Self::TokenApproval => "Token approval",
            Self::TokenTransfer => "Token transfer",
            Self::DenomTransfer => "Denom transfer",
            Self::LpTransfer => "LP transfer",
            Self::LpApproval => "LP approval",
            Self::PoolMint => "Pool mint",
            Self::PoolBurn => "Pool burn",
            Self::PoolSync => "Pool sync",
            Self::LiquidityUpdate => "Liquidity update",
            Self::PriceUpdate => "Price update",
            Self::TradingSimulation => "Trading simulation",
            Self::TaxSimulation => "Tax simulation",
            Self::Bribe => "Builder bribe",
            Self::ControlAddressActivity => "Control-address activity",
            Self::NetworkActivity => "Network activity",
            Self::Other => "Other tx",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Swap => "Pool swap with decoded token or denom flow.",
            Self::TokenApproval => "ERC-20 approval on the token contract.",
            Self::TokenTransfer => "ERC-20 transfer involving the token.",
            Self::DenomTransfer => "Transfer involving the pool denomination token.",
            Self::LpTransfer => "Liquidity-provider token transfer.",
            Self::LpApproval => "Liquidity-provider token approval.",
            Self::PoolMint => "Pool liquidity mint event.",
            Self::PoolBurn => "Pool liquidity burn event.",
            Self::PoolSync => "Pool reserve sync event.",
            Self::LiquidityUpdate => "Observed pool liquidity state update.",
            Self::PriceUpdate => "Observed pool price state update.",
            Self::TradingSimulation => "Buy/sell trading simulation result.",
            Self::TaxSimulation => "Buy/sell tax simulation result.",
            Self::Bribe => "Transaction paid a builder bribe.",
            Self::ControlAddressActivity => "Activity from a known owner/control address.",
            Self::NetworkActivity => "Activity from token-network context.",
            Self::Other => "Transaction did not match a more specific token analytics label.",
        }
    }

    pub fn affects_token(&self) -> bool {
        matches!(
            self,
            Self::Swap
                | Self::TokenApproval
                | Self::TokenTransfer
                | Self::TradingSimulation
                | Self::TaxSimulation
                | Self::ControlAddressActivity
        )
    }

    pub fn affects_pool(&self) -> bool {
        matches!(
            self,
            Self::Swap
                | Self::LpTransfer
                | Self::LpApproval
                | Self::PoolMint
                | Self::PoolBurn
                | Self::PoolSync
                | Self::LiquidityUpdate
                | Self::PriceUpdate
                | Self::TradingSimulation
                | Self::TaxSimulation
        )
    }

    fn priority(&self) -> u8 {
        match self {
            Self::Swap => 100,
            Self::PoolBurn => 96,
            Self::PoolMint => 94,
            Self::LiquidityUpdate => 90,
            Self::LpApproval => 84,
            Self::LpTransfer => 80,
            Self::TokenApproval => 74,
            Self::TokenTransfer => 70,
            Self::DenomTransfer => 66,
            Self::PoolSync => 62,
            Self::PriceUpdate => 60,
            Self::TradingSimulation => 56,
            Self::TaxSimulation => 54,
            Self::Bribe => 50,
            Self::ControlAddressActivity => 42,
            Self::NetworkActivity => 38,
            Self::Other => 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationTransactionClassification {
    pub primary_type: ObservationTransactionType,
    pub label: String,
    pub description: String,
    pub affects_token: bool,
    pub affects_pool: bool,
}

impl Default for ObservationTransactionClassification {
    fn default() -> Self {
        Self::from_types([ObservationTransactionType::Other])
    }
}

impl ObservationTransactionClassification {
    pub fn from_types(tx_types: impl IntoIterator<Item = ObservationTransactionType>) -> Self {
        let mut types: Vec<ObservationTransactionType> = tx_types.into_iter().collect();
        if types.is_empty() {
            types.push(ObservationTransactionType::Other);
        }
        let primary_type = types
            .iter()
            .max_by_key(|tx_type| tx_type.priority())
            .cloned()
            .unwrap_or(ObservationTransactionType::Other);
        let affects_token = types.iter().any(ObservationTransactionType::affects_token);
        let affects_pool = types.iter().any(ObservationTransactionType::affects_pool);

        Self {
            label: primary_type.label().to_string(),
            description: primary_type.description().to_string(),
            primary_type,
            affects_token,
            affects_pool,
        }
    }
}
