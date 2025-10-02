use alloy_primitives::Address;
/// Stablecoin Activity Detector
///
/// Detects mints and burns of stablecoins from transaction state changes.
/// Tracks 30+ stablecoins including USDC, USDT, DAI, and others.
use std::collections::HashMap;
use tracing::info;
// AddressStateChange is now part of tx_processor
use reth_chain_query::common_addresses::stablecoins::get_stablecoin_by_address;
use reth_chain_query::to_checksum_address;
use std::str::FromStr;
use tx_processor::tx_processor::data_models::AddressBalanceChange as AddressStateChange;

// No local stablecoin list: use reth_chain_query::common_addresses::stablecoins

#[derive(Debug, Clone)]
pub struct StablecoinSignal {
    pub signal_type: StablecoinActivityType,
    pub token_name: String,
    pub token_address: String,
    pub amount: f64,
    pub from_address: String,
    pub to_address: Option<String>,
    pub tx_hash: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StablecoinActivityType {
    Mint,
    Burn,
    LargeTransfer,
}

pub struct StablecoinDetector {}

impl StablecoinDetector {
    pub fn new() -> Self {
        Self {}
    }

    /// Detect stablecoin activity from state changes
    pub async fn detect(
        &self,
        tx_hash: &str,
        from_address: Address,
        to_address: Option<Address>,
        state_changes: &HashMap<Address, AddressStateChange>,
    ) -> Vec<StablecoinSignal> {
        let mut signals = Vec::new();

        // Check for stablecoin mints and burns
        for (address, changes) in state_changes {
            let address_str = to_checksum_address(address);

            // Check if this is a stablecoin address (via common_addresses)
            if get_stablecoin_by_address(*address).is_some() {
                // Look for balance changes in the stablecoin contract itself
                // This could indicate mints/burns
                for (token_addr, amount) in &changes.token_net {
                    if token_addr == &address_str {
                        // Self-referencing balance change could be internal accounting
                        continue;
                    }

                    // Check if the token being moved is also a stablecoin
                    if let Ok(parsed_addr) = Address::from_str(token_addr) {
                        if let Some(info) = get_stablecoin_by_address(parsed_addr) {
                            let token_name = info.symbol.to_string();
                            let decimals = info.decimals as i32;
                            let divisor = 10f64.powi(decimals);

                            // Convert U256 to f64 for analysis
                            // Note: U256 is always non-negative, represents absolute amounts
                            let amount_f64 =
                                amount.to_string().parse::<f64>().unwrap_or(0.0) / divisor;

                            // Determine activity type based on amount and context
                            let activity_type = if amount_f64 > 1.0 {
                                StablecoinActivityType::Mint
                            } else if amount_f64 < -1.0 {
                                StablecoinActivityType::Burn
                            } else {
                                continue; // Ignore small changes
                            };

                            let details = match activity_type {
                                StablecoinActivityType::Mint => {
                                    format!("{} minted: {:.6} tokens", token_name, amount_f64.abs())
                                }
                                StablecoinActivityType::Burn => {
                                    format!("{} burned: {:.6} tokens", token_name, amount_f64.abs())
                                }
                                StablecoinActivityType::LargeTransfer => format!(
                                    "{} large transfer: {:.6} tokens",
                                    token_name,
                                    amount_f64.abs()
                                ),
                            };

                            info!(
                                "💰 STABLECOIN {}: {} in tx {}",
                                match activity_type {
                                    StablecoinActivityType::Mint => "MINT",
                                    StablecoinActivityType::Burn => "BURN",
                                    StablecoinActivityType::LargeTransfer => "TRANSFER",
                                },
                                details,
                                tx_hash
                            );

                            signals.push(StablecoinSignal {
                                signal_type: activity_type,
                                token_name,
                                token_address: token_addr.clone(),
                                amount: amount_f64.abs(),
                                from_address: to_checksum_address(&from_address),
                                to_address: to_address.map(|a| to_checksum_address(&a)),
                                tx_hash: tx_hash.to_string(),
                                details,
                            });
                        }
                    }
                }
            }
        }

        signals
    }
}
