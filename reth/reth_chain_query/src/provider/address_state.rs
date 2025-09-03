/// Address state operations - balances, nonces, code, and portfolio management
/// 
/// Provides unified interface for all address-related state queries,
/// including ETH balances, token holdings, nonces, and contract detection.

use alloy_primitives::{Address, U256};
use eyre::Result;
use std::collections::HashMap;

use super::RethQueryProvider;

/// Represents a balance difference
#[derive(Debug, Clone)]
pub enum BalanceDiff {
    Increase(U256),
    Decrease(U256),
}

/// Represents a balance change between two blocks
#[derive(Debug, Clone)]
pub struct BalanceChange {
    pub before: U256,
    pub after: U256,
    pub difference: BalanceDiff,
}

/// Complete balance changes for an address between two blocks
#[derive(Debug, Clone)]
pub struct BalanceChanges {
    pub address: Address,
    pub from_block: u64,
    pub to_block: u64,
    pub eth_change: BalanceChange,
    pub token_changes: HashMap<Address, BalanceChange>,
}

/// Complete balances (ETH + tokens) at a specific block
#[derive(Debug, Clone)]
pub struct CompleteBalances {
    pub address: Address,
    pub eth_balance: U256,
    pub token_balances: HashMap<Address, U256>,
    pub block_number: u64,
}

impl RethQueryProvider {
    /// Get ETH balance for an address
    pub async fn get_eth_balance(&self, address: Address, block: Option<u64>) -> Result<U256> {
        let account = self.get_account(address, block).await?;
        Ok(account.balance)
    }
    
    /// Get complete balances (ETH + tokens) for an address
    pub async fn get_complete_balances(
        &self,
        address: Address,
        tokens: Vec<Address>,
        block: Option<u64>
    ) -> Result<CompleteBalances> {
        // Get ETH balance
        let eth_balance = self.get_eth_balance(address, block).await?;
        
        // Get token balances in parallel
        let token_balances = self.batch_get_balances_for_token_holder_pairs(
            tokens.iter().map(|&token| (token, address)).collect(),
            block
        ).await?;
        
        // Build token balance map (only non-zero)
        let mut token_map = HashMap::new();
        for (token, balance) in tokens.into_iter().zip(token_balances) {
            if balance > U256::ZERO {
                token_map.insert(token, balance);
            }
        }
        
        Ok(CompleteBalances {
            address,
            eth_balance,
            token_balances: token_map,
            block_number: block.unwrap_or(self.get_latest_block()?),
        })
    }
    
    /// Get ETH or token balance at a specific block (token=None for ETH)
    pub async fn get_eth_or_token_balance_at_block(
        &self,
        address: Address,
        token: Option<Address>,
        block: u64
    ) -> Result<U256> {
        match token {
            Some(token_addr) => self.get_token_balance(token_addr, address, Some(block)).await,
            None => self.get_eth_balance(address, Some(block)).await,
        }
    }
    
    /// Get balance changes between two blocks
    pub async fn calculate_eth_and_token_balance_diff_between_blocks(
        &self,
        address: Address,
        tokens: Vec<Address>,
        from_block: u64,
        to_block: u64
    ) -> Result<BalanceChanges> {
        // Get balances at both blocks
        let before = self.get_complete_balances(address, tokens.clone(), Some(from_block)).await?;
        let after = self.get_complete_balances(address, tokens, Some(to_block)).await?;
        
        // Calculate changes
        let eth_change = BalanceChange {
            before: before.eth_balance,
            after: after.eth_balance,
            difference: if after.eth_balance >= before.eth_balance {
                BalanceDiff::Increase(after.eth_balance - before.eth_balance)
            } else {
                BalanceDiff::Decrease(before.eth_balance - after.eth_balance)
            },
        };
        
        let mut token_changes = HashMap::new();
        
        // Check all tokens that existed in either period
        let all_tokens: std::collections::HashSet<_> = before.token_balances.keys()
            .chain(after.token_balances.keys())
            .copied()
            .collect();
        
        for token in all_tokens {
            let before_balance = before.token_balances.get(&token).copied().unwrap_or(U256::ZERO);
            let after_balance = after.token_balances.get(&token).copied().unwrap_or(U256::ZERO);
            
            if before_balance != after_balance {
                token_changes.insert(token, BalanceChange {
                    before: before_balance,
                    after: after_balance,
                    difference: if after_balance >= before_balance {
                        BalanceDiff::Increase(after_balance - before_balance)
                    } else {
                        BalanceDiff::Decrease(before_balance - after_balance)
                    },
                });
            }
        }
        
        Ok(BalanceChanges {
            address,
            from_block,
            to_block,
            eth_change,
            token_changes,
        })
    }
    
    
    /// Get transaction count (nonce) at a specific block
    pub async fn get_transaction_count_at_block(&self, address: Address, block: Option<u64>) -> Result<u64> {
        let account = self.get_account(address, block).await?;
        Ok(account.nonce)
    }
    
    /// Check if address has code (is a contract) - alias for is_contract
    pub async fn has_code(&self, address: Address, block: Option<u64>) -> Result<bool> {
        // Call the is_contract method from view_functions module
        let account = self.get_account(address, block).await?;
        Ok(account.code_hash.is_some())
    }
    
    /// Get contract bytecode at a specific block
    pub async fn get_contract_bytecode_at_block(&self, address: Address, block: Option<u64>) -> Result<Vec<u8>> {
        let block = block.unwrap_or(self.get_latest_block()?);
        let state = self.tx_simulator.get_chain_state_at_block(block)?;
        let account = state.basic_account(&address)?;
        
        if let Some(acc) = account {
            if acc.has_bytecode() {
                if let Some(bytecode) = state.bytecode_by_hash(&acc.bytecode_hash.unwrap_or(alloy_primitives::B256::ZERO))? {
                    return Ok(bytecode.bytecode().to_vec());
                }
            }
        }
        
        Ok(vec![])
    }
}