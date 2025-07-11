//! Block Data Fetcher - Gets block information and metadata

use qarqa_core_types::{QarqaError, QarqaResult, BlockNumber};
use sqlx::{PgPool, Row};
use tracing::{debug, error, info, warn};
use chrono::{DateTime, Utc};

/// Block data fetcher
pub struct BlockDataFetcher {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub number: BlockNumber,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: DateTime<Utc>,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub transaction_count: u32,
}

impl BlockDataFetcher {
    /// Create a new block data fetcher
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Get block information by number
    pub async fn get_block_info(&self, block_number: BlockNumber) -> QarqaResult<Option<BlockInfo>> {
        debug!("Fetching block info for block: {}", block_number);
        
        let query = "
            SELECT block_number, block_hash, parent_hash, block_timestamp, 
                   gas_limit, gas_used, transaction_count
            FROM blocks 
            WHERE block_number = $1
        ";
        
        let row = sqlx::query(query)
            .bind(block_number as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch block {}: {}", block_number, e);
                QarqaError::Database(e.to_string())
            })?;
        
        let Some(row) = row else {
            return Ok(None);
        };
        
        let number: i64 = row.try_get("block_number")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let hash: String = row.try_get("block_hash")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let parent_hash: String = row.try_get("parent_hash")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let timestamp: DateTime<Utc> = row.try_get("block_timestamp")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let gas_limit: i64 = row.try_get("gas_limit")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let gas_used: i64 = row.try_get("gas_used")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let transaction_count: i32 = row.try_get("transaction_count")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let block_info = BlockInfo {
            number: number as u64,
            hash,
            parent_hash,
            timestamp,
            gas_limit: gas_limit as u64,
            gas_used: gas_used as u64,
            transaction_count: transaction_count as u32,
        };
        
        debug!("Successfully fetched block info for: {}", block_number);
        Ok(Some(block_info))
    }
    
    /// Get latest block number
    pub async fn get_latest_block_number(&self) -> QarqaResult<BlockNumber> {
        debug!("Fetching latest block number");
        
        let query = "SELECT MAX(block_number) as latest_block FROM blocks";
        
        let row = sqlx::query(query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch latest block number: {}", e);
                QarqaError::Database(e.to_string())
            })?;
        
        let latest_block: Option<i64> = row.try_get("latest_block")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        match latest_block {
            Some(block) => {
                info!("Latest block number: {}", block);
                Ok(block as u64)
            }
            None => {
                warn!("No blocks found in database");
                Ok(0)
            }
        }
    }
    
    /// Get block range information
    pub async fn get_block_range(&self) -> QarqaResult<(BlockNumber, BlockNumber)> {
        debug!("Fetching block range");
        
        let query = "SELECT MIN(block_number) as min_block, MAX(block_number) as max_block FROM blocks";
        
        let row = sqlx::query(query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch block range: {}", e);
                QarqaError::Database(e.to_string())
            })?;
        
        let min_block: Option<i64> = row.try_get("min_block")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        let max_block: Option<i64> = row.try_get("max_block")
            .map_err(|e| QarqaError::Database(e.to_string()))?;
        
        match (min_block, max_block) {
            (Some(min), Some(max)) => {
                info!("Block range: {} - {}", min, max);
                Ok((min as u64, max as u64))
            }
            _ => {
                warn!("No block range found in database");
                Ok((0, 0))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_block_fetcher_basic() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
        
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
        let fetcher = BlockDataFetcher::new(pool);
        
        match fetcher.get_latest_block_number().await {
            Ok(latest) => println!("Latest block: {}", latest),
            Err(e) => println!("Database error: {}", e),
        }
        
        match fetcher.get_block_range().await {
            Ok((min, max)) => println!("Block range: {} - {}", min, max),
            Err(e) => println!("Database error: {}", e),
        }
    }
}