use std::collections::{HashMap, BTreeMap};
use ethers::types::Address;
use eyre::Result;

/// Rust equivalent of ProcessedTxStateDiffCalculator
/// 
/// Tracks and analyzes state changes in Ethereum transactions with exact same functionality:
/// 1. Tracking individual token movements per token contract address
/// 2. Tracking ETH/WETH movements 
/// 3. Handling special cases like WETH conversions and bribes
/// 4. Filtering out insignificant state changes based on thresholds
/// 
/// Key Features:
/// - Maintains chronological order of transfers using BTreeMap (OrderedDict equivalent)
/// - Tracks individual tokens separately with known symbols (USDC, USDT, DAI) or contract addresses
/// - Only applies decimal conversion for tokens with known decimals (no RPC calls)
/// - Tracks both incoming and outgoing movements for tokens and ETH
/// - Handles special addresses (WETH, null, dead addresses)
/// - Identifies significant state changes based on configurable thresholds
/// - Excludes WETH conversions from ETH movements
/// - Tracks bribe payments to known fee recipients
/// - Returns token_net as dictionary with symbols (stablecoins) or addresses (others)

// Known token symbols mapping
lazy_static::lazy_static! {
    static ref DENOM_ADDRESSES: HashMap<Address, &'static str> = {
        let mut m = HashMap::new();
        // Stablecoins
        m.insert("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(), "USDC");
        m.insert("0xdAC17F958D2ee523a2206206994597C13D831ec7".parse().unwrap(), "USDT");
        m.insert("0x6B175474E89094C44Da98b954EedeAC495271d0F".parse().unwrap(), "DAI");
        m.insert("0x4Fabb145d64652a948d72533023f6E7A623C7C53".parse().unwrap(), "BUSD");
        m.insert("0x8E870D67F660D95d5be530380D0eC0bd388289E1".parse().unwrap(), "PAX");
        m.insert("0x956F47F50A910163D8BF957Cf5846D573E7f87CA".parse().unwrap(), "FEI");
        m.insert("0x853d955aCEf822Db058eb8505911ED77F175b99e".parse().unwrap(), "FRAX");
        m.insert("0x5f98805A4E8be255a32880FDeC7F6728C6568bA0".parse().unwrap(), "LUSD");
        
        // Wrapped Tokens
        m.insert("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(), "WETH");
        m.insert("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".parse().unwrap(), "WBTC");
        m.insert("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse().unwrap(), "WBNB");
        
        // Other Major Tokens
        m.insert("0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0".parse().unwrap(), "MATIC");
        m.insert("0x514910771AF9Ca656af840dff83E8264EcF986CA".parse().unwrap(), "LINK");
        m.insert("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984".parse().unwrap(), "UNI");
        m.insert("0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9".parse().unwrap(), "AAVE");
        m
    };

    static ref ERC20_TOKEN_DECIMALS: HashMap<&'static str, u8> = {
        let mut m = HashMap::new();
        // Stablecoins
        m.insert("USDC", 6);
        m.insert("USDT", 6);
        m.insert("DAI", 18);
        m.insert("BUSD", 18);
        m.insert("PAX", 18);
        m.insert("FEI", 18);
        m.insert("FRAX", 18);
        m.insert("LUSD", 18);
        
        // Wrapped Tokens
        m.insert("WETH", 18);
        m.insert("WBTC", 8);
        m.insert("WBNB", 18);
        
        // Other Major Tokens
        m.insert("MATIC", 18);
        m.insert("LINK", 18);
        m.insert("UNI", 18);
        m.insert("AAVE", 18);
        m
    };

    // Known fee recipients (miners, MEV bots, etc.)
    static ref FEE_RECIPIENTS_SET: std::collections::HashSet<Address> = {
        let mut set = std::collections::HashSet::new();
        // Add common fee recipients
        set.insert("0x95222290dd7278aa3ddd389cc1e1d165cc4bafe5".parse().unwrap()); // Common validator
        set
    };
}

#[derive(Debug, Clone)]
pub struct MovementEntry {
    pub amount: f64,
}

#[derive(Debug, Clone)]
pub struct AddressMovements {
    pub incoming: BTreeMap<TransferId, MovementEntry>, // BTreeMap maintains order like OrderedDict
    pub outgoing: BTreeMap<TransferId, MovementEntry>,
}

impl Default for AddressMovements {
    fn default() -> Self {
        Self {
            incoming: BTreeMap::new(),
            outgoing: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransferId {
    pub block_number: u64,
    pub txn_index: u64,
    pub log_index_or_id: String,
}

impl TransferId {
    pub fn new(block_number: u64, txn_index: u64, log_index_or_id: impl Into<String>) -> Self {
        Self {
            block_number,
            txn_index,
            log_index_or_id: log_index_or_id.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddressStateChange {
    pub token_net: HashMap<String, f64>, // token_symbol_or_address -> net_change
    pub eth_net: f64,
    pub movements: AddressMovementsSummary,
}

#[derive(Debug, Clone)]
pub struct AddressMovementsSummary {
    pub tokens: HashMap<Address, AddressMovements>, // per token address
    pub denom: AddressMovements, // ETH movements
}

pub struct DebugTraceCallStateDiffCalculator {
    weth_address: Address,
    eth_state_change_threshold: f64,
    token_state_change_threshold: f64,
    
    // Track movements per address per token contract
    // Structure: movements[address][token_address] = AddressMovements
    denom_movements: HashMap<Address, AddressMovements>,
    token_movements: HashMap<Address, HashMap<Address, AddressMovements>>,
}

impl DebugTraceCallStateDiffCalculator {
    pub fn new(
        eth_state_change_threshold: f64,
        token_state_change_threshold: f64,
    ) -> Self {
        Self {
            weth_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(),
            eth_state_change_threshold,
            token_state_change_threshold,
            denom_movements: HashMap::new(),
            token_movements: HashMap::new(),
        }
    }

    /// Track a movement between addresses and handle special cases
    fn track_movement(
        &mut self,
        movement_type: MovementType,
        from_addr: Address,
        to_addr: Address,
        amount: f64,
        transfer_id: TransferId,
        token_address: Option<Address>,
    ) {
        // WETH conversions are denomination-neutral
        if to_addr == self.weth_address || from_addr == self.weth_address {
            return;
        }

        match movement_type {
            MovementType::Denom => {
                // Outgoing from `from_addr`
                self.denom_movements
                    .entry(from_addr)
                    .or_default()
                    .outgoing
                    .insert(transfer_id.clone(), MovementEntry { amount });

                // Incoming to `to_addr` (unless denom bribe to fee recipient)
                if !FEE_RECIPIENTS_SET.contains(&to_addr) {
                    self.denom_movements
                        .entry(to_addr)
                        .or_default()
                        .incoming
                        .insert(transfer_id, MovementEntry { amount });
                }
            }
            MovementType::Token => {
                if let Some(token_addr) = token_address {
                    // Outgoing from `from_addr`
                    self.token_movements
                        .entry(from_addr)
                        .or_default()
                        .entry(token_addr)
                        .or_default()
                        .outgoing
                        .insert(transfer_id.clone(), MovementEntry { amount });

                    // Incoming to `to_addr`
                    self.token_movements
                        .entry(to_addr)
                        .or_default()
                        .entry(token_addr)
                        .or_default()
                        .incoming
                        .insert(transfer_id, MovementEntry { amount });
                }
            }
        }
    }

    /// Aggregate net changes for every address touched in this tx
    pub fn get_net_changes(&self, from_address: Address) -> HashMap<Address, AddressStateChange> {
        let mut net = HashMap::new();

        // Get all addresses that had any movements
        let mut all_addrs = std::collections::HashSet::new();
        all_addrs.extend(self.denom_movements.keys());
        all_addrs.extend(self.token_movements.keys());

        for &addr in &all_addrs {
            // Calculate denom (ETH) net change
            let denom_movements = self.denom_movements.get(&addr);
            let denom_in: f64 = denom_movements
                .map(|m| m.incoming.values().map(|e| e.amount).sum())
                .unwrap_or(0.0);
            let denom_out: f64 = denom_movements
                .map(|m| m.outgoing.values().map(|e| e.amount).sum())
                .unwrap_or(0.0);
            let eth_net = denom_in - denom_out;

            // Calculate token net changes per token
            let mut token_net = HashMap::new();
            let mut total_token_movement = 0.0;

            if let Some(token_movements_for_addr) = self.token_movements.get(&addr) {
                for (&token_addr, movements) in token_movements_for_addr {
                    let token_in: f64 = movements.incoming.values().map(|e| e.amount).sum();
                    let token_out: f64 = movements.outgoing.values().map(|e| e.amount).sum();
                    let net_change = token_in - token_out;

                    if net_change.abs() > self.token_state_change_threshold {
                        // Use symbol for known stablecoins, address for others
                        let (token_key, net_change_formatted) = if let Some(&symbol) = DENOM_ADDRESSES.get(&token_addr) {
                            // Use symbol and apply decimal conversion if known
                            let formatted_change = if let Some(&decimals) = ERC20_TOKEN_DECIMALS.get(symbol) {
                                net_change / (10_f64.powi(decimals as i32))
                            } else {
                                net_change // Raw amount for unknown decimals
                            };
                            (symbol.to_string(), formatted_change)
                        } else {
                            // Use contract address for unknown tokens
                            (format!("{:#x}", token_addr), net_change)
                        };

                        token_net.insert(token_key, net_change_formatted);
                        total_token_movement += net_change_formatted.abs();
                    }
                }
            }

            // Include address if it meets thresholds or is the sender
            if total_token_movement > 0.0
                || eth_net.abs() > self.eth_state_change_threshold
                || addr == from_address
            {
                net.insert(
                    addr,
                    AddressStateChange {
                        token_net,
                        eth_net,
                        movements: AddressMovementsSummary {
                            tokens: self.token_movements.get(&addr).cloned().unwrap_or_default(),
                            denom: self.denom_movements.get(&addr).cloned().unwrap_or_default(),
                        },
                    },
                );
            }
        }

        net
    }

    /// Build movements from ERC20 transfers and ETH transfers
    pub fn calculate_state_changes_from_transfers(
        &mut self,
        from_address: Address,
        block_number: u64,
        txn_index: u64,
        eth_transfers: &[EthTransfer],
        erc20_transfers: &[Erc20Transfer],
    ) -> Result<HashMap<Address, AddressStateChange>> {
        // Clear previous state
        self.denom_movements.clear();
        self.token_movements.clear();

        // Process ERC-20 transfers
        for tr in erc20_transfers {
            let tid = TransferId::new(block_number, txn_index, tr.log_index.to_string());
            let is_weth = tr.token_address == self.weth_address;
            
            if is_weth {
                // WETH transfers treated as denom (ETH)
                let amount = tr.amount / 1e18;
                self.track_movement(
                    MovementType::Denom,
                    tr.from_address,
                    tr.to_address,
                    amount,
                    tid,
                    None,
                );
            } else {
                // Regular token transfers
                self.track_movement(
                    MovementType::Token,
                    tr.from_address,
                    tr.to_address,
                    tr.amount,
                    tid,
                    Some(tr.token_address),
                );
            }
        }

        // Process ETH (internal) transfers
        for tr in eth_transfers {
            let log_index_or_depth = tr.log_index.unwrap_or(tr.depth.unwrap_or(0));
            let tid = TransferId::new(block_number, txn_index, log_index_or_depth.to_string());
            self.track_movement(
                MovementType::Denom,
                tr.from_address,
                tr.to_address,
                tr.amount,
                tid,
                None,
            );
        }

        Ok(self.get_net_changes(from_address))
    }

    /// Reset state for new transaction
    pub fn reset(&mut self) {
        self.denom_movements.clear();
        self.token_movements.clear();
    }
}

impl Default for DebugTraceCallStateDiffCalculator {
    fn default() -> Self {
        Self::new(0.0005, 0.1) // Default thresholds from Python version
    }
}

#[derive(Debug, Clone, Copy)]
enum MovementType {
    Denom,
    Token,
}

#[derive(Debug, Clone)]
pub struct EthTransfer {
    pub from_address: Address,
    pub to_address: Address,
    pub amount: f64,
    pub log_index: Option<u64>,
    pub depth: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Erc20Transfer {
    pub token_address: Address,
    pub from_address: Address,
    pub to_address: Address,
    pub amount: f64,
    pub log_index: u64,
}