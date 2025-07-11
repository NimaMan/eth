//! Fast Path Integration for QARQA using revm_tx_simulator
//! 
//! Uses the FastPathProcessor from revm_tx_simulator to analyze transactions

use qarqa_core_types::{
    QarqaError, QarqaResult, Transaction, CompleteFundFlows,
    EthMovement, EthMovementType, TokenMovement
};
use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use tracing::{debug, info};
use std::str::FromStr;

use super::TransactionSimulator;

/// Fast path simulator using revm_tx_simulator's FastPathProcessor
pub struct FastPathSimulator {
    force_simulation: bool,
}

impl FastPathSimulator {
    pub fn new() -> Self {
        Self {
            force_simulation: true, // Always simulate for complete data
        }
    }
}

#[async_trait]
impl TransactionSimulator for FastPathSimulator {
    async fn initialize(&mut self) -> QarqaResult<()> {
        info!("Initializing Fast Path simulator");
        Ok(())
    }
    
    async fn simulate_transaction(&self, transaction: &Transaction) -> QarqaResult<CompleteFundFlows> {
        let tx_hash_str = format!("{:?}", transaction.hash);
        debug!("Analyzing transaction {} with FastPathProcessor", tx_hash_str);
        
        // Create fast path processor config
        let config = revm_tx_simulator_lib::FastPathConfig {
            force_full_simulation: self.force_simulation,
            complex_gas_threshold: 0, // Simulate everything
            cache_ttl_seconds: 300,
        };
        
        // Create processor
        let processor = revm_tx_simulator_lib::FastPathProcessor::new(config)
            .await
            .map_err(|e| QarqaError::Simulation(format!("Failed to create processor: {}", e)))?;
        
        // Analyze transaction
        let analysis = processor.analyze_transaction(&tx_hash_str)
            .await
            .map_err(|e| QarqaError::Simulation(format!("Analysis failed: {}", e)))?;
        
        debug!("Analysis complete: {} internal transfers detected", 
               analysis.internal_transfers_detected);
        
        // Parse the state changes to extract ALL transfers using revm_tx_simulator data
        let mut eth_movements = Vec::new();
        let mut token_movements = Vec::new();
        
        debug!("State changes structure: {:?}", analysis.state_changes);
        
        // Add direct ETH transfer if present
        if transaction.value > U256::ZERO {
            eth_movements.push(EthMovement {
                from: transaction.from_address,
                to: transaction.to_address.unwrap_or_default(),
                amount: transaction.value,
                movement_type: EthMovementType::Direct,
            });
        }
        
        // The FastPathProcessor returns simplified output, but we need the full state diff analysis
        // For now, let's extract what we can from the available data and add internal transfers
        // from the processor's internal_transfers field
        
        // Process internal transfers (these are the critical missing transfers!)
        if let Some(internal_transfers_value) = analysis.state_changes.get("internal_transfers") {
            if let Some(transfers_array) = internal_transfers_value.as_array() {
                for transfer in transfers_array {
                    if let (Some(from_str), Some(to_str), Some(value_str)) = (
                        transfer.get("from").and_then(|v| v.as_str()),
                        transfer.get("to").and_then(|v| v.as_str()),
                        transfer.get("value").and_then(|v| v.as_str()),
                    ) {
                        if let (Ok(from), Ok(to), Ok(amount)) = (
                            Address::from_str(from_str),
                            Address::from_str(to_str),
                            U256::from_str(value_str),
                        ) {
                            if amount > U256::ZERO {
                                eth_movements.push(EthMovement {
                                    from,
                                    to,
                                    amount,
                                    movement_type: EthMovementType::Internal,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // Extract token transfers from logs/events in the analysis
        // The revm_tx_simulator processes transaction receipt logs for ERC20 transfers
        if let Some(token_transfers) = analysis.state_changes.get("token_transfers") {
            if let Some(transfers_array) = token_transfers.as_array() {
                for transfer in transfers_array {
                    if let (Some(token_str), Some(from_str), Some(to_str), Some(amount_str)) = (
                        transfer.get("token").and_then(|v| v.as_str()),
                        transfer.get("from").and_then(|v| v.as_str()),
                        transfer.get("to").and_then(|v| v.as_str()),
                        transfer.get("amount").and_then(|v| v.as_str()),
                    ) {
                        if let (Ok(token_address), Ok(from), Ok(to), Ok(amount)) = (
                            Address::from_str(token_str),
                            Address::from_str(from_str),
                            Address::from_str(to_str),
                            U256::from_str(amount_str),
                        ) {
                            token_movements.push(TokenMovement {
                                token_address,
                                from,
                                to,
                                amount,
                                symbol: transfer.get("symbol").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                decimals: transfer.get("decimals").and_then(|v| v.as_u64()).map(|d| d as u8),
                            });
                        }
                    }
                }
            }
        }
        
        info!("Extracted {} ETH movements and {} token movements from REVM analysis", 
              eth_movements.len(), token_movements.len());
        
        // Add gas payment
        let gas_used = transaction.gas_used.unwrap_or(21000);
        eth_movements.push(EthMovement {
            from: transaction.from_address,
            to: Address::ZERO,
            amount: U256::from(gas_used) * transaction.gas_price,
            movement_type: EthMovementType::Gas,
        });
        
        Ok(CompleteFundFlows {
            transaction_hash: transaction.hash,
            block_number: transaction.block_number,
            timestamp: transaction.timestamp.unwrap_or_else(chrono::Utc::now),
            eth_movements,
            token_movements,
            gas_used,
            status: transaction.status,
        })
    }
}

impl Default for FastPathSimulator {
    fn default() -> Self {
        Self::new()
    }
}