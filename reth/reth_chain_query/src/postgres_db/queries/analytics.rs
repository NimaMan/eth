/// Analytics Queries
/// 
/// Complex analytical queries for ranking, network analysis, and aggregated metrics.

use crate::postgres_db::connection::PostgresDB;
use eyre::Result;
use sqlx::{query, Row};
use serde::{Deserialize, Serialize};

/// Address ranking result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressRanking {
    pub address: String,
    pub profit_rank: i32,
    pub volume_rank: i32,
    pub activity_rank: i32,
    pub composite_score: f64,
    pub bird_tier: String,  // Mapping to bird-themed ranking
}

/// Network relationship result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRelationship {
    pub from_address: String,
    pub to_address: String,
    pub interaction_count: i32,
    pub total_volume: f64,
    pub relationship_type: String,
}

/// Calculate address ranking based on multiple metrics
pub async fn calculate_address_ranking(
    db: &PostgresDB,
    limit: i64,
) -> Result<Vec<AddressRanking>> {
    let rows = query(
        r#"
        WITH ranked_addresses AS (
            SELECT 
                address,
                RANK() OVER (ORDER BY total_profit DESC NULLS LAST) as profit_rank,
                RANK() OVER (ORDER BY total_volume DESC NULLS LAST) as volume_rank,
                RANK() OVER (ORDER BY total_erc20_trades DESC NULLS LAST) as activity_rank,
                total_profit,
                total_volume,
                total_erc20_trades,
                scam_ratio
            FROM eth_db.addresses
            WHERE is_contract = false
        ),
        scored_addresses AS (
            SELECT 
                address,
                profit_rank,
                volume_rank,
                activity_rank,
                -- Composite score: weighted average of rankings
                (1.0 / NULLIF(profit_rank, 0) * 0.5 + 
                 1.0 / NULLIF(volume_rank, 0) * 0.3 + 
                 1.0 / NULLIF(activity_rank, 0) * 0.2) as composite_score
            FROM ranked_addresses
        )
        SELECT 
            address,
            profit_rank,
            volume_rank,
            activity_rank,
            composite_score,
            CASE 
                WHEN composite_score > 0.1 THEN 'Eagle'      -- Top tier
                WHEN composite_score > 0.05 THEN 'Hawk'      -- High tier
                WHEN composite_score > 0.01 THEN 'Falcon'    -- Mid-high tier
                WHEN composite_score > 0.005 THEN 'Owl'      -- Mid tier
                WHEN composite_score > 0.001 THEN 'Crow'     -- Mid-low tier
                ELSE 'Sparrow'                               -- Entry tier
            END as bird_tier
        FROM scored_addresses
        ORDER BY composite_score DESC
        LIMIT $1
        "#
    )
    .bind(limit)
    .fetch_all(db.pool())
    .await?;
    
    let rankings = rows.into_iter().map(|row| {
        AddressRanking {
            address: row.get("address"),
            profit_rank: row.get("profit_rank"),
            volume_rank: row.get("volume_rank"),
            activity_rank: row.get("activity_rank"),
            composite_score: row.get("composite_score"),
            bird_tier: row.get("bird_tier"),
        }
    }).collect();
    
    Ok(rankings)
}

/// Get network relationships between addresses
pub async fn get_network_relationships(
    db: &PostgresDB,
    address: &str,
    _depth: i32,
) -> Result<Vec<NetworkRelationship>> {
    // For now, we'll get direct relationships from trades
    // In a full implementation, this would traverse the related_addresses table
    let rows = query(
        r#"
        WITH address_trades AS (
            SELECT DISTINCT
                a1.address as from_address,
                a2.address as to_address,
                COUNT(*) as interaction_count,
                SUM(t.total_denom_spent + t.total_denom_received) as total_volume
            FROM eth_db.trades t
            JOIN eth_db.addresses a1 ON t.address_id = a1.address_id
            -- This is simplified - in reality we'd join through transactions
            JOIN eth_db.addresses a2 ON a2.address_id != a1.address_id
            WHERE a1.address = $1
            GROUP BY a1.address, a2.address
        )
        SELECT 
            from_address,
            to_address,
            interaction_count,
            total_volume,
            'trade_partner' as relationship_type
        FROM address_trades
        ORDER BY total_volume DESC
        LIMIT 100
        "#
    )
    .bind(address)
    .fetch_all(db.pool())
    .await?;
    
    let relationships = rows.into_iter().map(|row| {
        NetworkRelationship {
            from_address: row.get("from_address"),
            to_address: row.get("to_address"),
            interaction_count: row.get("interaction_count"),
            total_volume: row.get("total_volume"),
            relationship_type: row.get("relationship_type"),
        }
    }).collect();
    
    Ok(relationships)
}

/// Get aggregated statistics for the entire dataset
pub async fn get_global_statistics(db: &PostgresDB) -> Result<GlobalStats> {
    let row = query(
        r#"
        SELECT 
            COUNT(DISTINCT address_id) as total_addresses,
            COUNT(DISTINCT address_id) FILTER (WHERE is_contract = true) as total_contracts,
            COUNT(DISTINCT token_address) as total_tokens,
            COUNT(*) as total_trades,
            SUM(total_volume) as total_volume,
            SUM(total_profit) as total_profit,
            AVG(scam_ratio) as avg_scam_ratio,
            COUNT(DISTINCT address_id) FILTER (WHERE scam_ratio > 0.5) as high_scam_addresses
        FROM (
            SELECT 
                a.address_id,
                a.is_contract,
                a.total_volume,
                a.total_profit,
                a.scam_ratio,
                t.token_address
            FROM eth_db.addresses a
            LEFT JOIN eth_db.trades t ON a.address_id = t.address_id
        ) stats
        "#
    )
    .fetch_one(db.pool())
    .await?;
    
    Ok(GlobalStats {
        total_addresses: row.get("total_addresses"),
        total_contracts: row.get("total_contracts"),
        total_tokens: row.get("total_tokens"),
        total_trades: row.get("total_trades"),
        total_volume: row.get("total_volume"),
        total_profit: row.get("total_profit"),
        avg_scam_ratio: row.get("avg_scam_ratio"),
        high_scam_addresses: row.get("high_scam_addresses"),
    })
}

/// Global statistics structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStats {
    pub total_addresses: i64,
    pub total_contracts: i64,
    pub total_tokens: i64,
    pub total_trades: i64,
    pub total_volume: Option<f64>,
    pub total_profit: Option<f64>,
    pub avg_scam_ratio: Option<f64>,
    pub high_scam_addresses: i64,
}