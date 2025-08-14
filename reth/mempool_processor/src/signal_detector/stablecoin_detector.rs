/// Stablecoin Activity Detector
/// 
/// Detects mints and burns of stablecoins from transaction state changes.
/// Tracks 30+ stablecoins including USDC, USDT, DAI, and others.

use std::collections::{HashMap, HashSet};
use tracing::info;
use lazy_static::lazy_static;
use alloy_primitives::{Address, I256};
use reth_tx_simulator::AddressStateChange;
use crate::common::address::{alloy_address_to_checksum, checksum_address};

lazy_static! {
    /// Stablecoin addresses
    static ref STABLECOIN_ADDRESSES: HashSet<String> = {
        let mut addresses = HashSet::new();
        // Major stablecoins
        addresses.insert(checksum_address("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")); // USDC
        addresses.insert(checksum_address("0xdAC17F958D2ee523a2206206994597C13D831ec7")); // USDT
        addresses.insert(checksum_address("0x6B175474E89094C44Da98b954EedeAC495271d0F")); // DAI
        addresses.insert(checksum_address("0x4Fabb145d64652a948d72533023f6E7A623C7C53")); // BUSD
        addresses.insert(checksum_address("0x8E870D67F660D95d5be530380D0eC0bd388289E1")); // PAX
        addresses.insert(checksum_address("0x956F47F50A910163D8BF957Cf5846D573E7f87CA")); // FEI
        addresses.insert(checksum_address("0x853d955aCEf822Db058eb8505911ED77F175b99e")); // FRAX
        addresses.insert(checksum_address("0x5f98805A4E8be255a32880FDeC7F6728C6568bA0")); // LUSD
        // Wrapped stablecoins
        addresses.insert(checksum_address("0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643")); // cDAI
        addresses.insert(checksum_address("0x39AA39c021dfbaE8faC545936693aC917d5E7563")); // cUSDC
        // Additional stablecoins
        addresses.insert(checksum_address("0x0000000000085d4780B73119b644AE5ecd22b376")); // TUSD
        addresses.insert(checksum_address("0x674C6Ad92Fd080e4004b2312b45f796a192D27a0")); // USDN
        addresses.insert(checksum_address("0xe2f2a5C287993345a840Db3B0845fbC70f5935a5")); // mUSD
        addresses.insert(checksum_address("0x1456688345527bE1f37E9e627DA0837D6f08C925")); // USDP
        addresses.insert(checksum_address("0x57Ab1ec28D129707052df4dF418D58a2D46d5f51")); // sUSD
        addresses.insert(checksum_address("0x056Fd409E1d7A124BD7017459dFEa2F387b6d5Cd")); // GUSD
        addresses.insert(checksum_address("0xBC6DA0FE9aD5f3b0d58160288917AA56653660E9")); // alUSD
        addresses.insert(checksum_address("0x0E2EC54fC0B509F445631Bf4b91AB8168230C752")); // LINKUSD
        addresses.insert(checksum_address("0x83F20F44975D03b1b09e64809B757c47f942BEeA")); // sDAI
        addresses.insert(checksum_address("0x6c3ea9036406852006290770BEdFcAbA0e23A0e8")); // PYUSD
        addresses.insert(checksum_address("0x0C10bF8FcB7Bf5412187A595ab97a3609160b5c6")); // USDD
        addresses.insert(checksum_address("0xdC035D45d973E3EC169d2276DDab16f1e407384F")); // USDS
        addresses.insert(checksum_address("0x4DeA9e918c6289a52cd469cAC652727B7b412Cd2")); // USD0
        addresses.insert(checksum_address("0x605D26FBd5be761089281d5cec2Ce86eeA667109")); // USD0++
        addresses.insert(checksum_address("0x4c9EDD5852cd905f086C759E8383e09bff1E68B3")); // USDe
        addresses.insert(checksum_address("0x426E7d03f9803Dd11cb8616C65b99a3c0AfeA6dE")); // sUSDe
        addresses.insert(checksum_address("0xC52D7F23a2e460248Db6eE192Cb23dD12bDDCbf6")); // USD1
        addresses.insert(checksum_address("0x45fDb1b92a649fb6A64Ef1511D3Ba5Bf60044838")); // EURS
        addresses.insert(checksum_address("0xC581b735A1688071A1746c968e0798D642EDE491")); // EURT
        addresses.insert(checksum_address("0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c")); // EUROC
        addresses.insert(checksum_address("0x3231Cb76718CDeF2155FC47b5286d82e6eDA273f")); // EURCV
        addresses
    };
}

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
            let address_str = alloy_address_to_checksum(*address);
            
            // Check if this is a stablecoin address
            if STABLECOIN_ADDRESSES.contains(&address_str) {
                // Look for balance changes in the stablecoin contract itself
                // This could indicate mints/burns
                for (token_addr, amount) in &changes.token_net {
                    if token_addr == &address_str {
                        // Self-referencing balance change could be internal accounting
                        continue;
                    }
                    
                    // Check if the token being moved is also a stablecoin
                    if STABLECOIN_ADDRESSES.contains(token_addr) {
                        let token_name = self.get_stablecoin_name(token_addr);
                        let decimals = self.get_stablecoin_decimals(token_addr);
                        let divisor = 10f64.powi(decimals);
                        
                        // Convert I256 to f64 for analysis
                        let amount_f64 = if *amount >= I256::ZERO {
                            // Positive change (mint or receive)
                            amount.to_string().parse::<f64>().unwrap_or(0.0) / divisor
                        } else {
                            // Negative change (burn or send)
                            let abs_amount = (-*amount).to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                            -abs_amount
                        };
                        
                        // Determine activity type based on amount and context
                        let activity_type = if amount_f64 > 1.0 {
                            StablecoinActivityType::Mint
                        } else if amount_f64 < -1.0 {
                            StablecoinActivityType::Burn
                        } else {
                            continue; // Ignore small changes
                        };
                        
                        let details = match activity_type {
                            StablecoinActivityType::Mint => 
                                format!("{} minted: {:.6} tokens", token_name, amount_f64.abs()),
                            StablecoinActivityType::Burn => 
                                format!("{} burned: {:.6} tokens", token_name, amount_f64.abs()),
                            StablecoinActivityType::LargeTransfer => 
                                format!("{} large transfer: {:.6} tokens", token_name, amount_f64.abs()),
                        };
                        
                        info!("💰 STABLECOIN {}: {} in tx {}", 
                              match activity_type {
                                  StablecoinActivityType::Mint => "MINT",
                                  StablecoinActivityType::Burn => "BURN",
                                  StablecoinActivityType::LargeTransfer => "TRANSFER",
                              },
                              details, tx_hash);
                        
                        signals.push(StablecoinSignal {
                            signal_type: activity_type,
                            token_name,
                            token_address: token_addr.clone(),
                            amount: amount_f64.abs(),
                            from_address: alloy_address_to_checksum(from_address),
                            to_address: to_address.map(|a| alloy_address_to_checksum(a)),
                            tx_hash: tx_hash.to_string(),
                            details,
                        });
                    }
                }
            }
        }

        signals
    }

    /// Get stablecoin name from address
    fn get_stablecoin_name(&self, address: &str) -> String {
        match address {
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48" => "USDC",
            "0xdAC17F958D2ee523a2206206994597C13D831ec7" => "USDT",
            "0x6B175474E89094C44Da98b954EedeAC495271d0F" => "DAI",
            "0x4Fabb145d64652a948d72533023f6E7A623C7C53" => "BUSD",
            "0x8E870D67F660D95d5be530380D0eC0bd388289E1" => "PAX",
            "0x956F47F50A910163D8BF957Cf5846D573E7f87CA" => "FEI",
            "0x853d955aCEf822Db058eb8505911ED77F175b99e" => "FRAX",
            "0x5f98805A4E8be255a32880FDeC7F6728C6568bA0" => "LUSD",
            "0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643" => "cDAI",
            "0x39AA39c021dfbaE8faC545936693aC917d5E7563" => "cUSDC",
            "0x0000000000085d4780B73119b644AE5ecd22b376" => "TUSD",
            "0x674C6Ad92Fd080e4004b2312b45f796a192D27a0" => "USDN",
            "0xe2f2a5C287993345a840Db3B0845fbC70f5935a5" => "mUSD",
            "0x1456688345527bE1f37E9e627DA0837D6f08C925" => "USDP",
            "0x57ab1ec28d129707052df4df418d58a2d46d5f51" => "sUSD",
            "0x056fd409e1d7a124bd7017459dfea2f387b6d5cd" => "GUSD",
            "0xbc6da0fe9ad5f3b0d58160288917aa56653660e9" => "alUSD",
            "0x0e2ec54fc0b509f445631bf4b91ab8168230c752" => "LINKUSD",
            "0x83f20f44975d03b1b09e64809b757c47f942beea" => "sDAI",
            "0x6c3ea9036406852006290770bedfcaba0e23a0e8" => "PYUSD",
            "0x0c10bf8fcb7bf5412187a595ab97a3609160b5c6" => "USDD",
            "0xdc035d45d973e3ec169d2276ddab16f1e407384f" => "USDS",
            "0x4dea9e918c6289a52cd469cac652727b7b412cd2" => "USD0",
            "0x605d26fbd5be761089281d5cec2ce86eea667109" => "USD0++",
            "0x4c9edd5852cd905f086c759e8383e09bff1e68b3" => "USDe",
            "0x426e7d03f9803dd11cb8616c65b99a3c0afea6de" => "sUSDe",
            "0xc52d7f23a2e460248db6ee192cb23dd12bddcbf6" => "USD1",
            "0x45fdb1b92a649fb6a64ef1511d3ba5bf60044838" => "EURS",
            "0xc581b735a1688071a1746c968e0798d642ede491" => "EURT",
            "0x1abaea1f7c830bd89acc67ec4af516284b1bc33c" => "EUROC",
            "0x3231cb76718cdef2155fc47b5286d82e6eda273f" => "EURCV",
            _ => "Unknown Stablecoin",
        }.to_string()
    }

    /// Get stablecoin decimals
    fn get_stablecoin_decimals(&self, address: &str) -> i32 {
        match address {
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48" => 6,  // USDC
            "0xdAC17F958D2ee523a2206206994597C13D831ec7" => 6,  // USDT
            _ => 18, // Most use 18 decimals
        }
    }
}