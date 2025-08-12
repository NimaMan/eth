//! Fetcher for address-related data from eth_db

use crate::models::*;
use sqlx::PgPool;
use eyre::Result;
use tracing::{debug, info};

pub struct AddressFetcher {
    pool: PgPool,
}

impl AddressFetcher {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Get address record by address string
    pub async fn get_address(&self, address: &str) -> Result<Option<AddressRecord>> {
        let query = r#"
            SELECT address_id, address, is_contract,
                   total_profit, total_realized_profit, total_volume,
                   scam_ratio, trade_frequency, total_tx_fee,
                   first_seen, last_seen,
                   name, entity_category, cluster_label
            FROM eth_db.addresses
            WHERE address = $1
        "#;
        
        let record = sqlx::query_as::<_, AddressRecord>(query)
            .bind(address)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(record)
    }
    
    /// Get address record by address_id
    pub async fn get_address_by_id(&self, address_id: i64) -> Result<Option<AddressRecord>> {
        let query = r#"
            SELECT address_id, address, is_contract,
                   total_profit, total_realized_profit, total_volume,
                   scam_ratio, trade_frequency, total_tx_fee,
                   first_seen, last_seen,
                   name, entity_category, cluster_label
            FROM eth_db.addresses
            WHERE address_id = $1
        "#;
        
        let record = sqlx::query_as::<_, AddressRecord>(query)
            .bind(address_id)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(record)
    }
    
    /// Get all transactions for an address
    pub async fn get_address_transactions(
        &self, 
        address: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<AddressTransactions> {
        // First get the address record
        let address_record = self.get_address(address).await?
            .ok_or_else(|| eyre::eyre!("Address not found: {}", address))?;
        
        debug!("Found address_id {} for {}", address_record.address_id, address);
        
        // Get transaction count
        let count_query = r#"
            SELECT COUNT(DISTINCT tp.tx_hash) as count
            FROM eth_db.tx_participants tp
            WHERE tp.address_id = $1
        "#;
        
        let count: (i64,) = sqlx::query_as(count_query)
            .bind(address_record.address_id)
            .fetch_one(&self.pool)
            .await?;
        
        // Get transaction hashes
        let tx_query = r#"
            SELECT DISTINCT tp.tx_hash
            FROM eth_db.tx_participants tp
            JOIN eth_db.transactions t ON tp.tx_hash = t.tx_hash
            WHERE tp.address_id = $1
            ORDER BY t.block_number DESC
            LIMIT $2 OFFSET $3
        "#;
        
        let limit = limit.unwrap_or(1000);
        let offset = offset.unwrap_or(0);
        
        let tx_hashes: Vec<(String,)> = sqlx::query_as(tx_query)
            .bind(address_record.address_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        
        let tx_hashes: Vec<String> = tx_hashes.into_iter().map(|(hash,)| hash).collect();
        
        info!(
            "Found {} transactions for address {} (showing {} from offset {})",
            count.0, address, tx_hashes.len(), offset
        );
        
        Ok(AddressTransactions {
            address: address_record,
            tx_hashes,
            total_count: count.0,
        })
    }
    
    /// Get related addresses (fund flow connections)
    pub async fn get_related_addresses(
        &self,
        address: &str,
        min_flow: Option<f64>,
    ) -> Result<AddressRelationships> {
        let address_record = self.get_address(address).await?
            .ok_or_else(|| eyre::eyre!("Address not found: {}", address))?;
        
        let min_flow = min_flow.unwrap_or(0.01); // Default 0.01 ETH minimum
        
        // Get both incoming and outgoing relationships
        let query = r#"
            WITH relationships AS (
                -- Outgoing relationships
                SELECT ra.related_address_id as other_id, 
                       SUM(ra.denom_flow) as total_flow
                FROM eth_db.related_addresses ra
                WHERE ra.address_id = $1
                GROUP BY ra.related_address_id
                
                UNION ALL
                
                -- Incoming relationships
                SELECT ra.address_id as other_id,
                       SUM(ra.denom_flow) as total_flow
                FROM eth_db.related_addresses ra
                WHERE ra.related_address_id = $1
                GROUP BY ra.address_id
            )
            SELECT r.other_id, a.address, SUM(r.total_flow) as total_flow
            FROM relationships r
            JOIN eth_db.addresses a ON r.other_id = a.address_id
            WHERE ABS(r.total_flow) >= $2
            GROUP BY r.other_id, a.address
            ORDER BY ABS(total_flow) DESC
        "#;
        
        let related: Vec<(i64, String, f64)> = sqlx::query_as(query)
            .bind(address_record.address_id)
            .bind(min_flow)
            .fetch_all(&self.pool)
            .await?;
        
        info!(
            "Found {} related addresses for {} with flow >= {}",
            related.len(), address, min_flow
        );
        
        Ok(AddressRelationships {
            address: address_record,
            related_addresses: related,
        })
    }
    
    /// Get addresses by category (e.g., "CEX", "DEX", "EOA")
    pub async fn get_addresses_by_category(
        &self,
        category: &str,
        limit: Option<i64>,
    ) -> Result<Vec<AddressRecord>> {
        let query = r#"
            SELECT address_id, address, is_contract,
                   total_profit, total_realized_profit, total_volume,
                   scam_ratio, trade_frequency, total_tx_fee,
                   first_seen, last_seen,
                   name, entity_category, cluster_label
            FROM eth_db.addresses
            WHERE entity_category = $1
            ORDER BY total_volume DESC NULLS LAST
            LIMIT $2
        "#;
        
        let limit = limit.unwrap_or(100);
        
        let addresses = sqlx::query_as::<_, AddressRecord>(query)
            .bind(category)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        
        Ok(addresses)
    }
}