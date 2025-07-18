//! Database queries for graph discovery

use alloy_primitives::{Address, TxHash, U256};
use sqlx::{PgPool, FromRow};
use std::collections::HashMap;
use eyre::Result;

/// Transaction participant record from database
#[derive(Debug, FromRow)]
pub struct TxParticipant {
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub value: String,
    pub block_number: i64,
}

impl TxParticipant {
    /// Convert database strings to proper types
    pub fn into_typed(self) -> Result<TxParticipantTyped> {
        Ok(TxParticipantTyped {
            tx_hash: self.tx_hash.parse()?,
            from_address: self.from_address.parse()?,
            to_address: self.to_address.parse()?,
            value: U256::from_str_radix(&self.value, 10)?,
            block_number: self.block_number as u64,
        })
    }
}

pub struct TxParticipantTyped {
    pub tx_hash: TxHash,
    pub from_address: Address,
    pub to_address: Address,
    pub value: U256,
    pub block_number: u64,
}

/// Address information from database
#[derive(Debug, Default, FromRow)]
pub struct AddressInfo {
    pub address: String,
    pub is_contract: bool,
    pub cluster_label: Option<String>,
    pub scam_ratio: Option<f64>,
}

impl AddressInfo {
    pub fn into_typed(self) -> Result<AddressInfoTyped> {
        // Derive entity type before consuming self
        let entity_type = derive_entity_type(&self.cluster_label);
        
        Ok(AddressInfoTyped {
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
pub struct AddressInfoTyped {
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
    
    /// Get transactions involving an address - optimized approach
    pub async fn get_address_transactions(
        &self,
        address: &Address,
        min_value_wei: &U256,
        limit: usize,
        recent_blocks_only: Option<u64>,
    ) -> Result<Vec<TxParticipantTyped>> {
        let address_str = format!("{:?}", address);
        let min_value_eth = min_value_wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        
        // Step 1: Get address_id for our target address
        let address_id: i64 = sqlx::query_scalar(
            "SELECT address_id FROM eth_db.addresses WHERE LOWER(address) = LOWER($1)"
        )
        .bind(&address_str)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| eyre::eyre!("Address not found: {}", address_str))?;
        
        // Step 2: Get transaction hashes for this address
        let tx_hashes: Vec<String> = sqlx::query_scalar(
            "SELECT tx_hash FROM eth_db.tx_participants WHERE address_id = $1 LIMIT $2"
        )
        .bind(address_id)
        .bind(limit as i64 * 2) // Get more to filter
        .fetch_all(&self.pool)
        .await?;
        
        if tx_hashes.is_empty() {
            return Ok(Vec::new());
        }
        
        // Step 3: Get transaction details for these hashes
        let placeholders: Vec<String> = (1..=tx_hashes.len())
            .map(|i| format!("${}", i))
            .collect();
        
        let base_query = format!(
            r#"
            SELECT 
                t.tx_hash,
                a_from.address as from_address,
                COALESCE(a_to.address, '') as to_address,
                t.value::text as value,
                t.block_number::bigint as block_number
            FROM eth_db.transactions t
            JOIN eth_db.addresses a_from ON a_from.address_id = t.from_address_id
            LEFT JOIN eth_db.addresses a_to ON a_to.address_id = t.to_address_id
            WHERE t.tx_hash IN ({})
                AND t.value >= ${}
            {}
            ORDER BY t.value DESC, t.block_number DESC
            LIMIT ${}
            "#,
            placeholders.join(", "),
            tx_hashes.len() + 1,
            if recent_blocks_only.is_some() {
                format!("AND t.block_number >= ${}", tx_hashes.len() + 2)
            } else {
                String::new()
            },
            if recent_blocks_only.is_some() {
                tx_hashes.len() + 3
            } else {
                tx_hashes.len() + 2
            }
        );
        
        // Build and execute query
        let mut query_builder = sqlx::query_as::<_, TxParticipant>(&base_query);
        for tx_hash in &tx_hashes {
            query_builder = query_builder.bind(tx_hash);
        }
        query_builder = query_builder.bind(min_value_eth);
        
        if let Some(block_limit) = recent_blocks_only {
            query_builder = query_builder.bind(block_limit as i64);
        }
        
        query_builder = query_builder.bind(limit as i64);
        
        let rows = query_builder.fetch_all(&self.pool).await?;
        
        // Convert to typed
        let mut typed_results = Vec::new();
        for row in rows {
            let value_float = row.value.parse::<f64>().unwrap_or(0.0);
            let value_wei = U256::from((value_float * 1e18) as u128);
            
            typed_results.push(TxParticipantTyped {
                tx_hash: row.tx_hash.parse()?,
                from_address: row.from_address.parse()?,
                to_address: if row.to_address.is_empty() { 
                    Address::ZERO 
                } else { 
                    row.to_address.parse()? 
                },
                value: value_wei,
                block_number: row.block_number as u64,
            });
        }
        
        Ok(typed_results)
    }
    
    /// Get address metadata
    pub async fn get_address_info(
        &self,
        address: &Address,
    ) -> Result<AddressInfoTyped> {
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
        
        let info = sqlx::query_as::<_, AddressInfo>(query)
            .bind(&address_str)
            .fetch_optional(&self.pool)
            .await?
            .unwrap_or_else(|| AddressInfo {
                address: address_str.clone(),
                is_contract: false,
                cluster_label: None,
                scam_ratio: None,
            });
            
        info.into_typed()
    }
    
    /// Batch get address info for multiple addresses
    pub async fn get_addresses_info(
        &self,
        addresses: &[Address],
    ) -> Result<HashMap<Address, AddressInfoTyped>> {
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
        let mut query_builder = sqlx::query_as::<_, AddressInfo>(&query);
        for addr in addresses {
            let addr_str = format!("{:?}", addr).to_lowercase();
            query_builder = query_builder.bind(addr_str);
        }
        
        // Execute
        let rows = query_builder.fetch_all(&self.pool).await?;
        
        // Convert to HashMap
        let mut result = HashMap::new();
        for row in rows {
            if let Ok(typed) = row.into_typed() {
                result.insert(typed.address, typed);
            }
        }
        
        // Add default entries for addresses not in DB
        for addr in addresses {
            if !result.contains_key(addr) {
                result.insert(*addr, AddressInfoTyped {
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