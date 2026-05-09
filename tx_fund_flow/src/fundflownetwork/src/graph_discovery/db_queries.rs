//! Database queries for graph discovery

use alloy_primitives::{Address, TxHash, U256};
use sqlx::{PgPool, FromRow};
use std::collections::HashMap;
use eyre::Result;

/// Raw transaction participant record from database
#[derive(Debug, FromRow)]
pub struct RawTxParticipant {
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub value: String,
    pub block_number: i64,
}

impl RawTxParticipant {
    /// Parse database strings to proper types
    pub fn parse(self) -> Result<TxParticipant> {
        Ok(TxParticipant {
            tx_hash: self.tx_hash.parse()?,
            from_address: self.from_address.parse()?,
            to_address: self.to_address.parse()?,
            value: parse_transaction_value(&self.value)?,
            block_number: self.block_number as u64,
        })
    }
}

fn parse_transaction_value(value: &str) -> Result<U256> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(U256::ZERO);
    }

    if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
        let value = trimmed.parse::<f64>()?;
        if !value.is_finite() || value <= 0.0 {
            return Ok(U256::ZERO);
        }

        return Ok(U256::from(value.round() as u128));
    }

    Ok(U256::from_str_radix(trimmed, 10)?)
}

pub struct TxParticipant {
    pub tx_hash: TxHash,
    pub from_address: Address,
    pub to_address: Address,
    pub value: U256,
    pub block_number: u64,
}

/// Raw address information from database
#[derive(Debug, Default, FromRow)]
pub struct RawAddressInfo {
    pub address: String,
    pub is_contract: bool,
    pub cluster_label: Option<String>,
    pub scam_ratio: Option<f64>,
}

impl RawAddressInfo {
    pub fn parse(self) -> Result<AddressInfo> {
        // Derive entity type before consuming self
        let entity_type = derive_entity_type(&self.cluster_label);
        
        Ok(AddressInfo {
            address: self.address.parse()?,
            is_contract: self.is_contract,
            cluster_label: self.cluster_label,
            scam_ratio: self.scam_ratio,
            entity_type,
            name: None, // No name column in this schema
        })
    }
}

fn derive_entity_type(cluster_label: &Option<String>) -> Option<String> {
    // Map cluster labels to entity types
    match cluster_label.as_deref() {
        Some("CEX") => Some("CEX".to_string()),
        Some("DEX") => Some("DEX".to_string()),
        Some("BRIDGE") => Some("BRIDGE".to_string()),
        Some("MEV") => Some("MEV_BOT".to_string()),
        Some("WHALE") => Some("WHALE".to_string()),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub address: Address,
    pub is_contract: bool,
    pub cluster_label: Option<String>,
    pub scam_ratio: Option<f64>,
    pub entity_type: Option<String>,  // Derived from cluster_label
    pub name: Option<String>,         // Always None in this schema
}

/// Database queries for graph discovery
pub struct GraphDiscoveryQueries {
    pool: PgPool,
}

impl GraphDiscoveryQueries {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Get transactions involving an address - gets ALL participant pairs from tx_participants
    pub async fn get_address_transactions(
        &self,
        address: &Address,
        limit: usize,
        max_block: Option<u64>,
    ) -> Result<Vec<TxParticipant>> {
        let address_str = format!("{:?}", address);
        
        // Step 1: Get address_id for our target address
        let address_id: i64 = sqlx::query_scalar(
            "SELECT address_id FROM eth_db.addresses WHERE LOWER(address) = LOWER($1)"
        )
        .bind(&address_str)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| eyre::eyre!("Address not found: {}", address_str))?;
        
        // Step 2: Get transaction hashes for this address with block filtering
        let tx_hashes: Vec<(String, i32, String)> = if let Some(max_block) = max_block {
            sqlx::query_as(
                "SELECT DISTINCT tp.tx_hash, t.block_number, COALESCE(t.value::text, '0') AS value
                 FROM eth_db.tx_participants tp 
                 JOIN eth_db.transactions t ON t.tx_hash = tp.tx_hash
                 WHERE tp.address_id = $1 AND t.block_number <= $2 
                 ORDER BY t.block_number DESC 
                 LIMIT $3"
            )
            .bind(address_id)
            .bind(max_block as i32)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT DISTINCT tp.tx_hash, t.block_number, COALESCE(t.value::text, '0') AS value
                 FROM eth_db.tx_participants tp 
                 JOIN eth_db.transactions t ON t.tx_hash = tp.tx_hash
                 WHERE tp.address_id = $1 
                 ORDER BY t.block_number DESC 
                 LIMIT $2"
            )
            .bind(address_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?
        };
        
        if tx_hashes.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut all_participants = Vec::new();
        
        // Step 3: For each transaction, get ALL participant pairs
        for (tx_hash, block_number, value) in tx_hashes {
            let participants = self.get_tx_participants(&tx_hash.parse()?).await?;
            let value = parse_transaction_value(&value)?;
            
            // Create pairs between our target address and all other participants
            for participant in participants {
                if participant != *address {
                    all_participants.push(TxParticipant {
                        tx_hash: tx_hash.parse()?,
                        from_address: *address,
                        to_address: participant,
                        value,
                        block_number: block_number as u64,
                    });
                }
            }
        }
        
        Ok(all_participants)
    }
    
    /// Get address metadata
    pub async fn get_address_info(
        &self,
        address: &Address,
    ) -> Result<AddressInfo> {
        let address_str = format!("{:?}", address);
        
        let query = r#"
            SELECT 
                address,
                is_contract,
                cluster_label,
                scam_ratio
            FROM eth_db.addresses
            WHERE LOWER(address) = LOWER($1)
        "#;
        
        let info = sqlx::query_as::<_, RawAddressInfo>(query)
            .bind(&address_str)
            .fetch_optional(&self.pool)
            .await?
            .unwrap_or_else(|| RawAddressInfo {
                address: address_str.clone(),
                is_contract: false,
                cluster_label: None,
                scam_ratio: None,
            });
            
        info.parse()
    }
    
    /// Get all participants for a given transaction hash
    pub async fn get_tx_participants(
        &self,
        tx_hash: &TxHash,
    ) -> Result<Vec<Address>> {
        let tx_hash_str = format!("{:?}", tx_hash);
        
        let query = r#"
            SELECT DISTINCT a.address
            FROM eth_db.tx_participants tp
            JOIN eth_db.addresses a ON a.address_id = tp.address_id
            WHERE tp.tx_hash = $1
        "#;
        
        let addresses: Vec<String> = sqlx::query_scalar(query)
            .bind(&tx_hash_str)
            .fetch_all(&self.pool)
            .await?;
            
        let mut result = Vec::new();
        for addr_str in addresses {
            if let Ok(addr) = addr_str.parse() {
                result.push(addr);
            }
        }
        
        Ok(result)
    }

    /// Batch get address info for multiple addresses
    pub async fn get_addresses_info(
        &self,
        addresses: &[Address],
    ) -> Result<HashMap<Address, AddressInfo>> {
        if addresses.is_empty() {
            return Ok(HashMap::new());
        }
        
        // Build IN clause with placeholders
        let placeholders: Vec<String> = (1..=addresses.len())
            .map(|i| format!("${}", i))
            .collect();
        
        let query = format!(
            r#"
            SELECT address, is_contract, cluster_label, scam_ratio
            FROM eth_db.addresses
            WHERE LOWER(address) IN ({})
            "#,
            placeholders.join(", ")
        );
        
        // Build query
        let mut query_builder = sqlx::query_as::<_, RawAddressInfo>(&query);
        for addr in addresses {
            let addr_str = format!("{:?}", addr).to_lowercase();
            query_builder = query_builder.bind(addr_str);
        }
        
        // Execute
        let rows = query_builder.fetch_all(&self.pool).await?;
        
        // Convert to HashMap
        let mut result = HashMap::new();
        for row in rows {
            if let Ok(typed) = row.parse() {
                result.insert(typed.address, typed);
            }
        }
        
        // Add default entries for addresses not in DB
        for addr in addresses {
            if !result.contains_key(addr) {
                result.insert(*addr, AddressInfo {
                    address: *addr,
                    is_contract: false,
                    cluster_label: None,
                    scam_ratio: None,
                    entity_type: None,
                    name: None,
                });
            }
        }
        
        Ok(result)
    }
}
