use eth_alpha_core::amount::DecimalAmount;

#[derive(Clone, Debug, PartialEq)]
pub struct EntryInitPolicyConfig {
    /// Reject when the entry block is more than this many blocks after pool
    /// creation. None disables the age threshold.
    pub max_age_blocks: Option<u64>,
    /// Reject when creation block is unavailable. This is useful once live
    /// inputs reliably carry creation evidence.
    pub require_creation_block: bool,
    /// Reject when current pool price / initial pool price is above this value.
    /// None disables the price-ratio threshold.
    pub max_price_ratio_to_initial: Option<DecimalAmount>,
    /// Missing price-ratio evidence does not block entry while true.
    pub allow_missing_price_ratio: bool,
}

impl Default for EntryInitPolicyConfig {
    fn default() -> Self {
        Self {
            max_age_blocks: None,
            require_creation_block: false,
            max_price_ratio_to_initial: None,
            allow_missing_price_ratio: true,
        }
    }
}
