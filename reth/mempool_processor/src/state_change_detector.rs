// State change detection module for mempool processor
// Tracks balance changes and pool interactions

use ethers::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use eyre::Result;

#[derive(Debug, Clone)]
pub struct StateChange {
    pub address: Address,
    pub token: Address,
    pub balance_before: U256,
    pub balance_after: U256,
    pub change: I256,
}

#[derive(Debug, Clone)]
pub struct EthStateChange {
    pub address: Address,
    pub balance_before: U256,
    pub balance_after: U256,
    pub change: I256,
}

#[derive(Debug, Clone)]
pub struct StateChanges {
    pub eth_changes: Vec<EthStateChange>,
    pub token_changes: Vec<StateChange>,
}

#[derive(Debug, Clone)]
pub struct PoolInteraction {
    pub pool_address: Address,
    pub token_in: Address,
    pub amount_in: U256,
    pub token_out: Address,
    pub amount_out: U256,
    pub interaction_type: PoolInteractionType,
}

#[derive(Debug, Clone)]
pub enum PoolInteractionType {
    Swap,
    AddLiquidity,
    RemoveLiquidity,
}

pub struct StateChangeDetector {
    provider: Arc<Provider<Http>>,
    erc20_abi: ethers::abi::Abi,
    weth_address: Address,
}

impl StateChangeDetector {
    pub fn new(provider: Provider<Http>) -> Result<Self> {
        // Minimal ERC20 ABI for balance queries
        let erc20_abi = serde_json::from_str(r#"[
            {
                "constant": true,
                "inputs": [{"name": "_owner", "type": "address"}],
                "name": "balanceOf",
                "outputs": [{"name": "balance", "type": "uint256"}],
                "type": "function"
            }
        ]"#)?;
        
        // WETH address on mainnet
        let weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
        
        Ok(Self {
            provider: Arc::new(provider),
            erc20_abi,
            weth_address,
        })
    }
    
    /// Extract comprehensive state changes from a transaction (ETH and tokens)
    pub async fn extract_comprehensive_state_changes(
        &self,
        tx_hash: H256,
        watched_addresses: &[Address],
    ) -> Result<StateChanges> {
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?
            .ok_or_else(|| eyre::eyre!("Transaction receipt not found"))?;
            
        let block_before = receipt.block_number.unwrap() - 1;
        let block_after = receipt.block_number.unwrap();
        
        let mut eth_changes = Vec::new();
        let mut token_changes = Vec::new();
        
        // Parse transfer events
        let transfers = self.parse_transfer_events(&receipt.logs);
        
        // Track ETH movements (including WETH transfers)
        let mut eth_movements: HashMap<Address, I256> = HashMap::new();
        
        // Process transfers
        for transfer in &transfers {
            // Check if this is a WETH transfer
            if transfer.token == self.weth_address {
                // Track as ETH movement
                let amount_eth = I256::try_from(transfer.amount / U256::from(10u64.pow(18)))?;
                
                // Subtract from sender
                *eth_movements.entry(transfer.from).or_insert(I256::zero()) -= amount_eth;
                // Add to receiver
                *eth_movements.entry(transfer.to).or_insert(I256::zero()) += amount_eth;
            } else {
                // Regular token transfer - only track if it involves watched addresses
                if watched_addresses.contains(&transfer.from) || 
                   watched_addresses.contains(&transfer.to) ||
                   watched_addresses.contains(&transfer.token) {
                    
                    // Check balance changes for involved addresses
                    for addr in [transfer.from, transfer.to] {
                        if let Ok(change) = self.get_balance_change(
                            transfer.token,
                            addr,
                            block_before.as_u64(),
                            block_after.as_u64()
                        ).await {
                            if change.change != I256::zero() {
                                token_changes.push(change);
                            }
                        }
                    }
                }
            }
        }
        
        // Add internal ETH transfers from transaction
        if let Some(tx) = self.provider.get_transaction(tx_hash).await? {
            if let Some(to) = tx.to {
                if tx.value > U256::zero() {
                    let amount_eth = I256::try_from(tx.value / U256::from(10u64.pow(18)))?;
                    *eth_movements.entry(tx.from).or_insert(I256::zero()) -= amount_eth;
                    *eth_movements.entry(to).or_insert(I256::zero()) += amount_eth;
                }
            }
        }
        
        // Convert ETH movements to state changes for watched addresses
        for (addr, net_change) in eth_movements {
            if watched_addresses.contains(&addr) && net_change != I256::zero() {
                // For simplicity, we're just tracking net changes
                // In production, you'd query actual before/after balances
                eth_changes.push(EthStateChange {
                    address: addr,
                    balance_before: U256::zero(), // Would need to query
                    balance_after: U256::zero(),   // Would need to query
                    change: net_change,
                });
            }
        }
        
        Ok(StateChanges {
            eth_changes,
            token_changes,
        })
    }
    
    /// Extract state changes from a transaction (legacy method for compatibility)
    pub async fn extract_state_changes(
        &self,
        tx_hash: H256,
        watched_addresses: &[Address],
    ) -> Result<Vec<StateChange>> {
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?
            .ok_or_else(|| eyre::eyre!("Transaction receipt not found"))?;
            
        let block_before = receipt.block_number.unwrap() - 1;
        let block_after = receipt.block_number.unwrap();
        
        let mut state_changes = Vec::new();
        
        // Parse transfer events to identify tokens and addresses
        let transfers = self.parse_transfer_events(&receipt.logs);
        
        // Check balance changes for each unique token-address pair
        for (token, address) in self.get_unique_token_address_pairs(&transfers, watched_addresses) {
            if let Ok(change) = self.get_balance_change(
                token,
                address,
                block_before.as_u64(),
                block_after.as_u64()
            ).await {
                if change.change != I256::zero() {
                    state_changes.push(change);
                }
            }
        }
        
        Ok(state_changes)
    }
    
    /// Detect pool interactions from transfer patterns
    pub fn detect_pool_interactions(
        &self,
        logs: &[Log],
        pool_addresses: &[Address],
    ) -> Vec<PoolInteraction> {
        let transfers = self.parse_transfer_events(logs);
        let mut interactions = Vec::new();
        
        // Group transfers by address
        let mut address_transfers: HashMap<Address, Vec<Transfer>> = HashMap::new();
        for transfer in transfers {
            address_transfers.entry(transfer.from).or_default().push(transfer.clone());
            address_transfers.entry(transfer.to).or_default().push(transfer.clone());
        }
        
        // Check each pool address
        for &pool_addr in pool_addresses {
            if let Some(pool_transfers) = address_transfers.get(&pool_addr) {
                // Separate incoming and outgoing transfers
                let incoming: Vec<_> = pool_transfers.iter()
                    .filter(|t| t.to == pool_addr)
                    .collect();
                let outgoing: Vec<_> = pool_transfers.iter()
                    .filter(|t| t.from == pool_addr)
                    .collect();
                    
                // Detect swap pattern: pool receives one token and sends another
                if incoming.len() == 1 && outgoing.len() == 1 {
                    let token_in = incoming[0].token;
                    let token_out = outgoing[0].token;
                    
                    if token_in != token_out {
                        interactions.push(PoolInteraction {
                            pool_address: pool_addr,
                            token_in,
                            amount_in: incoming[0].amount,
                            token_out,
                            amount_out: outgoing[0].amount,
                            interaction_type: PoolInteractionType::Swap,
                        });
                    }
                }
                // Detect liquidity patterns (simplified)
                else if incoming.len() == 2 && outgoing.is_empty() {
                    // Add liquidity: pool receives two tokens
                    // This is simplified - real detection would check for LP token minting
                    if incoming[0].token != incoming[1].token {
                        interactions.push(PoolInteraction {
                            pool_address: pool_addr,
                            token_in: incoming[0].token,
                            amount_in: incoming[0].amount,
                            token_out: incoming[1].token,
                            amount_out: incoming[1].amount,
                            interaction_type: PoolInteractionType::AddLiquidity,
                        });
                    }
                }
            }
        }
        
        interactions
    }
    
    /// Get balance change for a token-address pair
    async fn get_balance_change(
        &self,
        token: Address,
        holder: Address,
        block_before: u64,
        block_after: u64,
    ) -> Result<StateChange> {
        let contract = Contract::new(token, self.erc20_abi.clone(), self.provider.clone());
        
        // Get balance before
        let balance_before: U256 = contract
            .method::<_, U256>("balanceOf", holder)?
            .block(block_before)
            .call()
            .await?;
            
        // Get balance after
        let balance_after: U256 = contract
            .method::<_, U256>("balanceOf", holder)?
            .block(block_after)
            .call()
            .await?;
            
        // Calculate change
        let change = if balance_after >= balance_before {
            I256::try_from(balance_after - balance_before)?
        } else {
            -I256::try_from(balance_before - balance_after)?
        };
        
        Ok(StateChange {
            address: holder,
            token,
            balance_before,
            balance_after,
            change,
        })
    }
    
    /// Parse transfer events from logs
    fn parse_transfer_events(&self, logs: &[Log]) -> Vec<Transfer> {
        let transfer_topic = H256::from_slice(
            &ethers::core::utils::keccak256("Transfer(address,address,uint256)")
        );
        
        let mut transfers = Vec::new();
        
        for log in logs {
            if log.topics.len() >= 3 && log.topics[0] == transfer_topic {
                let from = Address::from_slice(&log.topics[1].as_bytes()[12..]);
                let to = Address::from_slice(&log.topics[2].as_bytes()[12..]);
                let amount = U256::from_big_endian(&log.data);
                
                // Skip WETH deposit/withdrawal (transfers to/from WETH contract itself)
                if from == self.weth_address || to == self.weth_address {
                    continue;
                }
                
                transfers.push(Transfer {
                    token: log.address,
                    from,
                    to,
                    amount,
                });
            }
        }
        
        transfers
    }
    
    /// Get unique token-address pairs from transfers
    fn get_unique_token_address_pairs(
        &self,
        transfers: &[Transfer],
        watched_addresses: &[Address],
    ) -> Vec<(Address, Address)> {
        let mut pairs = std::collections::HashSet::new();
        
        for transfer in transfers {
            // Check if any watched address is involved
            if watched_addresses.contains(&transfer.from) {
                pairs.insert((transfer.token, transfer.from));
            }
            if watched_addresses.contains(&transfer.to) {
                pairs.insert((transfer.token, transfer.to));
            }
            if watched_addresses.contains(&transfer.token) {
                // If watching the token itself, track all addresses
                pairs.insert((transfer.token, transfer.from));
                pairs.insert((transfer.token, transfer.to));
            }
        }
        
        pairs.into_iter().collect()
    }
}

#[derive(Debug, Clone)]
struct Transfer {
    token: Address,
    from: Address,
    to: Address,
    amount: U256,
}

/// Example usage in main.rs
pub async fn analyze_transaction_state_changes(
    provider: Provider<Http>,
    tx_hash: H256,
    replicandy_token: Address,
    replicandy_pool: Address,
) -> Result<()> {
    let detector = StateChangeDetector::new(provider.clone())?;
    
    // Addresses to watch
    let watched_addresses = vec![replicandy_token, replicandy_pool];
    
    // Extract state changes
    let state_changes = detector.extract_state_changes(tx_hash, &watched_addresses).await?;
    
    println!("\nState Changes Detected:");
    for change in &state_changes {
        println!("  Address: {:?}", change.address);
        println!("  Token: {:?}", change.token);
        println!("  Before: {}", change.balance_before);
        println!("  After: {}", change.balance_after);
        println!("  Change: {}", change.change);
        println!();
    }
    
    // Get transaction receipt for pool interaction detection
    if let Some(receipt) = provider.get_transaction_receipt(tx_hash).await? {
        let pool_interactions = detector.detect_pool_interactions(&receipt.logs, &[replicandy_pool]);
        
        if !pool_interactions.is_empty() {
            println!("Pool Interactions Detected:");
            for interaction in &pool_interactions {
                println!("  Pool: {:?}", interaction.pool_address);
                println!("  Type: {:?}", interaction.interaction_type);
                println!("  Token In: {:?} Amount: {}", interaction.token_in, interaction.amount_in);
                println!("  Token Out: {:?} Amount: {}", interaction.token_out, interaction.amount_out);
            }
        }
    }
    
    Ok(())
}