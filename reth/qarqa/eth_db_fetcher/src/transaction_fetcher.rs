//! Fetcher for transaction data from eth_db

use crate::models::*;
use sqlx::{PgPool, Row};
use eyre::Result;
use tracing::{debug, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct TransactionFetcher {
    pool: PgPool,
    address_id_cache: Arc<RwLock<HashMap<String, i64>>>,
}

impl TransactionFetcher {
    pub fn new(pool: PgPool) -> Self {
        Self { 
            pool,
            address_id_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: &str) -> Result<Option<TransactionRecord>> {
        let query = r#"
            SELECT tx_hash, block_number, value, status, from_address_id, to_address_id
            FROM eth_db.transactions
            WHERE tx_hash = $1
        "#;
        
        let record = sqlx::query_as::<_, TransactionRecord>(query)
            .bind(tx_hash)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(record)
    }
    
    /// Get transactions by block number
    pub async fn get_block_transactions(
        &self,
        block_number: i64,
        limit: Option<i64>,
    ) -> Result<Vec<TransactionRecord>> {
        let query = r#"
            SELECT tx_hash, block_number, position_in_block,
                   from_address, to_address, value, gas_limit, gas_price,
                   gas_used, status, input_data, receipt_status,
                   cumulative_gas_used, effective_gas_price
            FROM eth_db.transactions
            WHERE block_number = $1
            ORDER BY position_in_block
            LIMIT $2
        "#;
        
        let limit = limit.unwrap_or(10000);
        
        let records = sqlx::query_as::<_, TransactionRecord>(query)
            .bind(block_number)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        
        Ok(records)
    }
    
    /// Get recent transactions
    pub async fn get_recent_transactions(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<TransactionRecord>> {
        let query = r#"
            SELECT tx_hash, block_number, position_in_block,
                   from_address, to_address, value, gas_limit, gas_price,
                   gas_used, status, input_data, receipt_status,
                   cumulative_gas_used, effective_gas_price
            FROM eth_db.transactions
            ORDER BY block_number DESC, position_in_block DESC
            LIMIT $1
        "#;
        
        let limit = limit.unwrap_or(100);
        
        let records = sqlx::query_as::<_, TransactionRecord>(query)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        
        Ok(records)
    }
    
    /// Get high value transactions
    pub async fn get_high_value_transactions(
        &self,
        min_value_eth: f64,
        limit: Option<i64>,
    ) -> Result<Vec<TransactionRecord>> {
        // Convert ETH to Wei (as string to handle large numbers)
        let min_value_wei = format!("{}", (min_value_eth * 1e18) as u128);
        
        let query = r#"
            SELECT tx_hash, block_number, position_in_block,
                   from_address, to_address, value, gas_limit, gas_price,
                   gas_used, status, input_data, receipt_status,
                   cumulative_gas_used, effective_gas_price
            FROM eth_db.transactions
            WHERE value::numeric >= $1::numeric
            ORDER BY value::numeric DESC
            LIMIT $2
        "#;
        
        let limit = limit.unwrap_or(100);
        
        let records = sqlx::query_as::<_, TransactionRecord>(query)
            .bind(&min_value_wei)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        
        info!("Found {} high value transactions >= {} ETH", records.len(), min_value_eth);
        
        Ok(records)
    }
    
    /// Get transactions between two addresses
    pub async fn get_transactions_between_addresses(
        &self,
        from_address: &str,
        to_address: &str,
        limit: Option<i64>,
    ) -> Result<Vec<TransactionRecord>> {
        let query = r#"
            SELECT tx_hash, block_number, position_in_block,
                   from_address, to_address, value, gas_limit, gas_price,
                   gas_used, status, input_data, receipt_status,
                   cumulative_gas_used, effective_gas_price
            FROM eth_db.transactions
            WHERE from_address = $1 AND to_address = $2
            ORDER BY block_number DESC
            LIMIT $3
        "#;
        
        let limit = limit.unwrap_or(100);
        
        let records = sqlx::query_as::<_, TransactionRecord>(query)
            .bind(from_address)
            .bind(to_address)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        
        debug!("Found {} transactions from {} to {}", 
               records.len(), from_address, to_address);
        
        Ok(records)
    }
    
    /// PERFORMANCE-OPTIMIZED: Get transaction hashes and blocks for an address 
    /// Uses address_id directly and minimal query for maximum speed
    pub async fn get_address_transactions_fast(
        &self,
        address_id: i64,
        max_block: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<AddressTransaction>> {
        let limit = limit.unwrap_or(1000);
        
        // Direct query with address_id - uses optimal indexes
        let records = match max_block {
            Some(block) => {
                sqlx::query_as::<_, AddressTransaction>(r#"
                    SELECT t.tx_hash, t.block_number
                    FROM eth_db.tx_participants tp
                    INNER JOIN eth_db.transactions t ON tp.tx_hash = t.tx_hash
                    WHERE tp.address_id = $1 AND t.block_number <= $2
                    ORDER BY t.block_number DESC
                    LIMIT $3
                "#)
                .bind(address_id)
                .bind(block)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, AddressTransaction>(r#"
                    SELECT t.tx_hash, t.block_number
                    FROM eth_db.tx_participants tp
                    INNER JOIN eth_db.transactions t ON tp.tx_hash = t.tx_hash
                    WHERE tp.address_id = $1
                    ORDER BY t.block_number DESC
                    LIMIT $2
                "#)
                .bind(address_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };
        
        Ok(records)
    }

    /// Get address_id for an address (with aggressive caching for repeated lookups)
    pub async fn get_address_id(&self, address: &str) -> Result<Option<i64>> {
        // Check cache first (read lock)
        {
            let cache = self.address_id_cache.read().await;
            if let Some(&address_id) = cache.get(address) {
                return Ok(Some(address_id));
            }
        }
        
        // Cache miss - query database
        let address_id: Option<i64> = sqlx::query_scalar(
            "SELECT address_id FROM eth_db.addresses WHERE address = $1"
        )
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;
        
        // Cache the result (write lock)
        if let Some(id) = address_id {
            let mut cache = self.address_id_cache.write().await;
            cache.insert(address.to_string(), id);
        }
        
        Ok(address_id)
    }

    /// BACKWARD COMPATIBILITY: Original method with address string lookup
    pub async fn get_address_transactions(
        &self,
        address: &str,
        max_block: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<AddressTransaction>> {
        let address_id = match self.get_address_id(address).await? {
            Some(id) => id,
            None => {
                debug!("Address {} not found in database", address);
                return Ok(vec![]);
            }
        };
        
        self.get_address_transactions_fast(address_id, max_block, limit).await
    }

    /// BATCH OPERATION: Get address_ids for multiple addresses at once
    pub async fn get_address_ids_batch(&self, addresses: &[String]) -> Result<HashMap<String, i64>> {
        let mut result = HashMap::new();
        let mut uncached_addresses = Vec::new();
        
        // Check cache for all addresses first
        {
            let cache = self.address_id_cache.read().await;
            for address in addresses {
                if let Some(&address_id) = cache.get(address) {
                    result.insert(address.clone(), address_id);
                } else {
                    uncached_addresses.push(address.clone());
                }
            }
        }
        
        // Batch query for uncached addresses
        if !uncached_addresses.is_empty() {
            let batch_query = format!(
                "SELECT address, address_id FROM eth_db.addresses WHERE address = ANY($1)"
            );
            
            let rows = sqlx::query(&batch_query)
                .bind(&uncached_addresses)
                .fetch_all(&self.pool)
                .await?;
            
            // Cache and collect results
            let mut cache = self.address_id_cache.write().await;
            for row in rows {
                let address: String = row.get("address");
                let address_id: i64 = row.get("address_id");
                cache.insert(address.clone(), address_id);
                result.insert(address, address_id);
            }
        }
        
        Ok(result)
    }
    
    /// ULTRA-FAST: Get transactions for multiple addresses at once
    pub async fn get_address_transactions_batch(
        &self,
        address_ids: &[i64],
        max_block: Option<i64>,
        limit: Option<i64>,
    ) -> Result<HashMap<i64, Vec<AddressTransaction>>> {
        let limit = limit.unwrap_or(1000);
        
        let query = match max_block {
            Some(_) => r#"
                SELECT tp.address_id, t.tx_hash, t.block_number
                FROM eth_db.tx_participants tp
                INNER JOIN eth_db.transactions t ON tp.tx_hash = t.tx_hash
                WHERE tp.address_id = ANY($1) AND t.block_number <= $2
                ORDER BY tp.address_id, t.block_number DESC
            "#,
            None => r#"
                SELECT tp.address_id, t.tx_hash, t.block_number
                FROM eth_db.tx_participants tp
                INNER JOIN eth_db.transactions t ON tp.tx_hash = t.tx_hash
                WHERE tp.address_id = ANY($1)
                ORDER BY tp.address_id, t.block_number DESC
            "#,
        };
        
        let rows = match max_block {
            Some(block) => {
                sqlx::query(query)
                    .bind(address_ids)
                    .bind(block)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query(query)
                    .bind(address_ids)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        
        // Group results by address_id
        let mut result: HashMap<i64, Vec<AddressTransaction>> = HashMap::new();
        for row in rows {
            let address_id: i64 = row.get("address_id");
            let tx_hash: String = row.get("tx_hash");
            let block_number: i32 = row.get("block_number");
            
            let tx = AddressTransaction { tx_hash, block_number };
            result.entry(address_id).or_insert_with(Vec::new).push(tx);
        }
        
        // Apply limit per address
        for (_, txs) in result.iter_mut() {
            txs.truncate(limit as usize);
        }
        
        Ok(result)
    }

    /// Get transaction count statistics for a date range
    pub async fn get_transaction_stats(
        &self,
        start_block: i64,
        end_block: i64,
    ) -> Result<TransactionStats> {
        let query = r#"
            SELECT 
                COUNT(*) as total_count,
                SUM(CASE WHEN status = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN status = 0 THEN 1 ELSE 0 END) as failed_count,
                AVG(gas_used) as avg_gas_used,
                MIN(block_number) as first_block,
                MAX(block_number) as last_block
            FROM eth_db.transactions
            WHERE block_number BETWEEN $1 AND $2
        "#;
        
        let stats: (i64, Option<i64>, Option<i64>, Option<f64>, Option<i64>, Option<i64>) = 
            sqlx::query_as(query)
                .bind(start_block)
                .bind(end_block)
                .fetch_one(&self.pool)
                .await?;
        
        Ok(TransactionStats {
            total_count: stats.0,
            success_count: stats.1.unwrap_or(0),
            failed_count: stats.2.unwrap_or(0),
            avg_gas_used: stats.3.unwrap_or(0.0),
            block_range: (stats.4.unwrap_or(start_block), stats.5.unwrap_or(end_block)),
        })
    }
}

/// Transaction statistics
#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub total_count: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub avg_gas_used: f64,
    pub block_range: (i64, i64),
}