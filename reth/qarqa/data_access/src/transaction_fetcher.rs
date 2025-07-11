//! Transaction Data Fetcher - Gets transaction details by hash

use qarqa_core_types::{QarqaError, QarqaResult, Transaction, TransactionHash};
use alloy_primitives::{Address, B256, U256, hex};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use tracing::{debug, error, info};
use chrono::{DateTime, Utc};

/// Transaction data fetcher
pub struct TransactionDataFetcher {
    pool: PgPool,
}

impl TransactionDataFetcher {
    /// Create a new transaction data fetcher
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Get transaction by hash
    pub async fn get_transaction_by_hash(&self, hash: TransactionHash) -> QarqaResult<Option<Transaction>> {
        debug!("Fetching transaction: {:?}", hash);
        
        let query = "
            SELECT t.hash, t.block_number, t.from_address, t.to_address, t.value, 
                   t.gas_limit, t.gas_price, t.input_data, t.status, t.gas_used,
                   b.timestamp
            FROM transactions t
            LEFT JOIN blocks b ON t.block_number = b.block_number  
            WHERE t.hash = $1
        ";
        
        let row = sqlx::query(query)
            .bind(format!("{:?}", hash))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch transaction {}: {}", hash, e);
                QarqaError::Database(e.to_string())
            })?;
        
        let Some(row) = row else {
            return Ok(None);
        };
        
        let tx_hash_str: String = row.try_get("hash")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        let tx_hash = B256::from_str(&tx_hash_str)
            .map_err(|e| QarqaError::AddressParsing(format!("Invalid transaction hash: {}", e)))?;
        
        let block_number: i64 = row.try_get("block_number")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let from_address_str: String = row.try_get("from_address")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        let from_address = Address::from_str(&from_address_str)
            .map_err(|e| QarqaError::AddressParsing(format!("Invalid from address: {}", e)))?;
        
        let to_address_str: Option<String> = row.try_get("to_address")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        let to_address = match to_address_str {
            Some(addr_str) => Some(Address::from_str(&addr_str)
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid to address: {}", e)))?),
            None => None,
        };
        
        let value_str: String = row.try_get("value")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        let value = U256::from_str(&value_str)
            .map_err(|e| QarqaError::AddressParsing(format!("Invalid value: {}", e)))?;
        
        let gas_limit: i64 = row.try_get("gas_limit")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let gas_price_str: String = row.try_get("gas_price")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        let gas_price = U256::from_str(&gas_price_str)
            .map_err(|e| QarqaError::AddressParsing(format!("Invalid gas price: {}", e)))?;
        
        let input_data_hex: String = row.try_get("input_data")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        let input_data = hex::decode(input_data_hex.trim_start_matches("0x"))
            .map_err(|e| QarqaError::AddressParsing(format!("Invalid input data: {}", e)))?;
        
        let status: bool = row.try_get("status")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let gas_used: Option<i64> = row.try_get("gas_used")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let timestamp: Option<DateTime<Utc>> = row.try_get("timestamp")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let transaction = Transaction {
            hash: tx_hash,
            block_number: block_number as u64,
            from_address,
            to_address,
            value,
            gas_limit: gas_limit as u64,
            gas_price,
            input_data,
            status,
            gas_used: gas_used.map(|g| g as u64),
            timestamp,
        };
        
        debug!("Successfully fetched transaction: {:?}", hash);
        Ok(Some(transaction))
    }
    
    /// Get multiple transactions by hash (batch operation)
    pub async fn get_transactions_by_hashes(&self, hashes: &[TransactionHash]) -> QarqaResult<Vec<Transaction>> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }
        
        debug!("Fetching {} transactions", hashes.len());
        
        let hash_strings: Vec<String> = hashes.iter()
            .map(|h| format!("{:?}", h))
            .collect();
        
        let query = "
            SELECT t.hash, t.block_number, t.from_address, t.to_address, t.value,
                   t.gas_limit, t.gas_price, t.input_data, t.status, t.gas_used,
                   b.timestamp
            FROM transactions t
            LEFT JOIN blocks b ON t.block_number = b.block_number
            WHERE t.hash = ANY($1)
        ";
        
        let rows = sqlx::query(query)
            .bind(&hash_strings)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch transactions: {}", e);
                QarqaError::Database(e.to_string())
            })?;
        
        let mut transactions = Vec::new();
        
        for row in rows {
            // Parse each row (same logic as get_transaction_by_hash)
            let tx_hash_str: String = row.try_get("hash")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let tx_hash = B256::from_str(&tx_hash_str)
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid transaction hash: {}", e)))?;
            
            let block_number: i64 = row.try_get("block_number")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            let from_address_str: String = row.try_get("from_address")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let from_address = Address::from_str(&from_address_str)
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid from address: {}", e)))?;
            
            let to_address_str: Option<String> = row.try_get("to_address")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let to_address = match to_address_str {
                Some(addr_str) => Some(Address::from_str(&addr_str)
                    .map_err(|e| QarqaError::AddressParsing(format!("Invalid to address: {}", e)))?),
                None => None,
            };
            
            let value_str: String = row.try_get("value")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let value = U256::from_str(&value_str)
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid value: {}", e)))?;
            
            let gas_limit: i64 = row.try_get("gas_limit")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            let gas_price_str: String = row.try_get("gas_price")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let gas_price = U256::from_str(&gas_price_str)
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid gas price: {}", e)))?;
            
            let input_data_hex: String = row.try_get("input_data")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let input_data = hex::decode(input_data_hex.trim_start_matches("0x"))
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid input data: {}", e)))?;
            
            let status: bool = row.try_get("status")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            let gas_used: Option<i64> = row.try_get("gas_used")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            let timestamp: Option<DateTime<Utc>> = row.try_get("timestamp")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            transactions.push(Transaction {
                hash: tx_hash,
                block_number: block_number as u64,
                from_address,
                to_address,
                value,
                gas_limit: gas_limit as u64,
                gas_price,
                input_data,
                status,
                gas_used: gas_used.map(|g| g as u64),
                timestamp,
            });
        }
        
        info!("Successfully fetched {} transactions", transactions.len());
        Ok(transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_transaction_fetcher_basic() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
        
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
        let fetcher = TransactionDataFetcher::new(pool);
        
        // Test with a known transaction hash
        let hash = B256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
        
        match fetcher.get_transaction_by_hash(hash).await {
            Ok(Some(tx)) => println!("Found transaction: {:?}", tx.hash),
            Ok(None) => println!("Transaction not found (expected)"),
            Err(e) => println!("Database error: {}", e),
        }
    }
}