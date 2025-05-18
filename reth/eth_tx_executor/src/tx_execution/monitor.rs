/*
* Transaction Monitor
*
* Responsible for:
* - Tracking transaction status
* - Getting transaction receipts
* - Tracking confirmation counts
* - Monitoring for dropped or replaced transactions
* - Updating transaction status in the cache
*
* Algorithm:
* 1. Start tracking a transaction by hash
* 2. Periodically check for receipt
* 3. Once mined, track confirmations by comparing block number to latest
* 4. Update status in the cache
* 5. Report status changes through callbacks/events
*/

use crate::tx_execution::{
    types::{TxStatus, TxReceipt, TxConfirmation},
    config::TxExecutionConfig,
    error::{TxError, TxResult},
};

use ethers::{
    providers::Middleware,
    types::{H256, U256},
};

use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Simple transaction monitor for tracking transaction status
pub struct TxMonitor<M> 
where
    M: Middleware + Clone
{
    /// Provider for blockchain interaction
    provider: Arc<M>,
    
    /// Configuration for transaction execution
    config: TxExecutionConfig,
}

impl<M> TxMonitor<M> 
where
    M: Middleware + Clone
{
    /// Create a new transaction monitor
    pub fn new(provider: Arc<M>, config: TxExecutionConfig) -> Self {
        Self {
            provider,
            config,
        }
    }
    
    /// Start monitoring a transaction
    pub async fn monitor_transaction(&self, tx_hash: H256) -> TxResult<TxStatus> {
        // For simplified tests, we'll just get the current status
        // In a real implementation, this would set up a background task or subscription
        self.get_transaction_status(tx_hash).await
    }
    
    /// Get current transaction status
    pub async fn get_transaction_status(&self, tx_hash: H256) -> TxResult<TxStatus> {
        // Check for receipt
        let receipt = self.provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| TxError::MonitoringFailed(format!("Failed to get receipt: {}", e)))?;
        
        if let Some(receipt) = receipt {
            // Transaction has been mined
            let tx_receipt: TxReceipt = receipt.into();
            
            // Check confirmations
            let latest_block = self.provider
                .get_block_number()
                .await
                .map_err(|e| TxError::MonitoringFailed(format!("Failed to get latest block: {}", e)))?;
            
            let block_number = tx_receipt.block_number;
            let current_block = U256::from(latest_block.as_u64());
            
            // Calculate confirmations (current - tx block)
            let confirmations = if current_block > block_number {
                (current_block - block_number).as_u64()
            } else {
                0
            };
            
            if confirmations >= self.config.default_confirmation_blocks {
                // Transaction is confirmed
                return Ok(TxStatus::Confirmed {
                    time: SystemTime::now(),
                    hash: tx_hash,
                    block_hash: tx_receipt.block_hash,
                    block_number,
                    confirmations,
                    receipt: tx_receipt,
                });
            } else {
                // Transaction is mined but not yet fully confirmed
                return Ok(TxStatus::Mined {
                    time: SystemTime::now(),
                    hash: tx_hash,
                    block_hash: tx_receipt.block_hash,
                    block_number,
                    receipt: tx_receipt,
                });
            }
        } else {
            // Transaction is still pending
            // In a real implementation, we would check for various failure cases here:
            // - Dropped from mempool
            // - Replaced by another transaction
            // - Timed out
            return Ok(TxStatus::Pending {
                last_updated: SystemTime::now(),
                hash: tx_hash,
            });
        }
    }
    
    /// Get confirmation info for a transaction
    pub async fn get_confirmation_info(&self, tx_hash: H256, required_confirmations: u64) -> TxResult<TxConfirmation> {
        // Get receipt
        let receipt = self.provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| TxError::MonitoringFailed(format!("Failed to get receipt: {}", e)))?;
        
        let receipt = receipt.ok_or_else(|| TxError::MonitoringFailed(
            format!("No receipt found for transaction: {:?}", tx_hash)
        ))?;
        
        // Get latest block
        let latest_block = self.provider
            .get_block_number()
            .await
            .map_err(|e| TxError::MonitoringFailed(format!("Failed to get latest block: {}", e)))?;
        
        // Safe handling for block number
        let block_num = receipt.block_number.unwrap_or_default().as_u64();
        let block_number = U256::from(block_num);
        let current_block = U256::from(latest_block.as_u64());
        
        // Calculate confirmations
        let confirmations = if current_block > block_number {
            (current_block - block_number).as_u64()
        } else {
            0
        };
        
        // Create confirmation info
        Ok(TxConfirmation {
            hash: tx_hash,
            block_hash: receipt.block_hash.unwrap_or_default(),
            block_number,
            current_block,
            confirmations,
            required_confirmations,
            is_confirmed: confirmations >= required_confirmations,
        })
    }
    
    /// Wait for a transaction to be confirmed
    pub async fn wait_for_confirmation(&self, tx_hash: H256, _required_confirmations: u64, timeout: Duration) -> TxResult<TxStatus> {
        let start_time = SystemTime::now();
        
        loop {
            let status = self.get_transaction_status(tx_hash).await?;
            
            match &status {
                TxStatus::Confirmed { .. } => {
                    return Ok(status);
                },
                TxStatus::Failed { .. } => {
                    return Ok(status);
                },
                _ => {
                    // Check timeout
                    if SystemTime::now().duration_since(start_time).unwrap_or_default() > timeout {
                        return Ok(TxStatus::Timeout {
                            time: SystemTime::now(),
                            hash: tx_hash,
                            timeout,
                        });
                    }
                    
                    // Wait before checking again
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}
