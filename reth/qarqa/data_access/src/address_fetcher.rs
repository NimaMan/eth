//! Address Data Fetcher - Leverages participants table for O(1) lookups

use qarqa_core_types::{QarqaError, QarqaResult, TransactionParticipant, TransactionHash, BlockNumber, ParticipantDirection};
use alloy_primitives::{Address, B256};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use tracing::{debug, error, info};

/// Address data fetcher - leverages your participants table
pub struct AddressDataFetcher {
    pool: PgPool,
}

impl AddressDataFetcher {
    /// Create a new address data fetcher
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Get transaction hashes for an address (O(1) using participants table)
    pub async fn get_address_transactions(
        &self,
        address: Address,
        start_block: Option<BlockNumber>,
        end_block: Option<BlockNumber>, 
        limit: Option<u64>,
    ) -> QarqaResult<Vec<TransactionHash>> {
        debug!("Fetching transactions for address: {:?}", address);
        
        let mut query = String::from(
            "SELECT DISTINCT transaction_hash FROM address_transactions WHERE address = $1"
        );
        
        let mut param_count = 1;
        
        // Add block range filters
        if start_block.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND block_number >= ${}", param_count));
        }
        
        if end_block.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND block_number <= ${}", param_count));
        }
        
        query.push_str(" ORDER BY block_number DESC");
        
        if limit.is_some() {
            param_count += 1;
            query.push_str(&format!(" LIMIT ${}", param_count));
        }
        
        let mut sql_query = sqlx::query(&query);
        sql_query = sql_query.bind(format!("{:?}", address));
        
        if let Some(start) = start_block {
            sql_query = sql_query.bind(start as i64);
        }
        
        if let Some(end) = end_block {
            sql_query = sql_query.bind(end as i64);
        }
        
        if let Some(lim) = limit {
            sql_query = sql_query.bind(lim as i64);
        }
        
        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch address transactions: {}", e);
                QarqaError::Database(e.to_string())
            })?;
        
        let mut tx_hashes = Vec::new();
        for row in rows {
            let tx_hash_str: String = row.try_get("transaction_hash")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            let tx_hash = B256::from_str(&tx_hash_str)
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid transaction hash: {}", e)))?;
            
            tx_hashes.push(tx_hash);
        }
        
        info!("Found {} transactions for address {:?}", tx_hashes.len(), address);
        Ok(tx_hashes)
    }
    
    /// Get transaction participants with metadata
    pub async fn get_transaction_participants(
        &self,
        address: Address,
        start_block: Option<BlockNumber>,
        end_block: Option<BlockNumber>,
    ) -> QarqaResult<Vec<TransactionParticipant>> {
        debug!("Fetching transaction participants for address: {:?}", address);
        
        let mut query = String::from(
            "SELECT address, transaction_hash, block_number, direction, value_change, token_transfers 
             FROM address_transactions WHERE address = $1"
        );
        
        let mut param_count = 1;
        
        if start_block.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND block_number >= ${}", param_count));
        }
        
        if end_block.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND block_number <= ${}", param_count));
        }
        
        query.push_str(" ORDER BY block_number DESC");
        
        let mut sql_query = sqlx::query(&query);
        sql_query = sql_query.bind(format!("{:?}", address));
        
        if let Some(start) = start_block {
            sql_query = sql_query.bind(start as i64);
        }
        
        if let Some(end) = end_block {
            sql_query = sql_query.bind(end as i64);
        }
        
        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let mut participants = Vec::new();
        
        for row in rows {
            let address_str: String = row.try_get("address")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let address = Address::from_str(&address_str)
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid address: {}", e)))?;
            
            let tx_hash_str: String = row.try_get("transaction_hash")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let transaction_hash = B256::from_str(&tx_hash_str)
                .map_err(|e| QarqaError::AddressParsing(format!("Invalid transaction hash: {}", e)))?;
            
            let block_number: i64 = row.try_get("block_number")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            let direction_str: Option<String> = row.try_get("direction")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            let direction = match direction_str.as_deref() {
                Some("in") => ParticipantDirection::In,
                Some("out") => ParticipantDirection::Out,
                Some("both") => ParticipantDirection::Both,
                _ => ParticipantDirection::Both, // Default
            };
            
            let value_change: Option<i64> = row.try_get("value_change")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            let token_transfers: Option<serde_json::Value> = row.try_get("token_transfers")
                .map_err(|e| QarqaError::Database(e.to_string()))?;
            
            participants.push(TransactionParticipant {
                address,
                transaction_hash,
                block_number: block_number as u64,
                direction,
                value_change,
                token_transfers,
            });
        }
        
        info!("Found {} participants for address {:?}", participants.len(), address);
        Ok(participants)
    }
    
    /// Get transaction count for an address
    pub async fn get_transaction_count(
        &self,
        address: Address,
        start_block: Option<BlockNumber>,
        end_block: Option<BlockNumber>,
    ) -> QarqaResult<u64> {
        debug!("Getting transaction count for address: {:?}", address);
        
        let mut query = String::from(
            "SELECT COUNT(DISTINCT transaction_hash) as count FROM address_transactions WHERE address = $1"
        );
        
        let mut param_count = 1;
        
        if start_block.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND block_number >= ${}", param_count));
        }
        
        if end_block.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND block_number <= ${}", param_count));
        }
        
        let mut sql_query = sqlx::query(&query);
        sql_query = sql_query.bind(format!("{:?}", address));
        
        if let Some(start) = start_block {
            sql_query = sql_query.bind(start as i64);
        }
        
        if let Some(end) = end_block {
            sql_query = sql_query.bind(end as i64);
        }
        
        let row = sql_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let count: i64 = row.try_get("count")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_address_fetcher_basic() {
        // This test requires a real database connection
        // Run with: cargo test -- --ignored
        
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
        
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
        let fetcher = AddressDataFetcher::new(pool);
        
        // Test with a known address
        let address = Address::from_str("0x0000000000000000000000000000000000000000").unwrap();
        
        match fetcher.get_transaction_count(address, None, None).await {
            Ok(count) => println!("Transaction count: {}", count),
            Err(e) => println!("Expected error (no participants table): {}", e),
        }
    }
}