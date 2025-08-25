/// Token Queries
/// 
/// Queries for token metadata, pools, and scam detection.

use crate::postgres_db::{connection::PostgresDB, models::{Token, Pool}};
use eyre::Result;
use sqlx::query_as;

/// Get token information by contract address
pub async fn get_token_info(db: &PostgresDB, token_address: &str) -> Result<Option<Token>> {
    let token = query_as::<_, Token>(
        r#"
        SELECT 
            contract_address,
            creator_address_id,
            is_scam,
            scam_label,
            creation_txn,
            trading_enabled_txn
        FROM eth_db.tokens
        WHERE contract_address = $1
        "#
    )
    .bind(token_address)
    .fetch_optional(db.pool())
    .await?;
    
    Ok(token)
}

/// Get all pools for a token across DEX protocols
pub async fn get_token_pools(db: &PostgresDB, token_address: &str) -> Result<Vec<Pool>> {
    let pools = query_as::<_, Pool>(
        r#"
        SELECT 
            id,
            pool_address,
            pool_id,
            pool_type,
            token_address,
            pair_token_address,
            fee_tier,
            is_scam,
            scam_label,
            scam_block,
            scam_tx_hash,
            trading_enabled,
            trading_enabled_block,
            trading_enabled_txn
        FROM eth_db.pools
        WHERE token_address = $1
        ORDER BY pool_type, fee_tier
        "#
    )
    .bind(token_address)
    .fetch_all(db.pool())
    .await?;
    
    Ok(pools)
}

/// Get all scam tokens
pub async fn get_scam_tokens(db: &PostgresDB, limit: Option<i64>) -> Result<Vec<Token>> {
    let mut sql = String::from(
        r#"
        SELECT 
            contract_address,
            creator_address_id,
            is_scam,
            scam_label,
            creation_txn,
            trading_enabled_txn
        FROM eth_db.tokens
        WHERE is_scam = true
        "#
    );
    
    if let Some(lim) = limit {
        sql.push_str(&format!(" LIMIT {}", lim));
    }
    
    let tokens = sqlx::query_as::<_, Token>(&sql)
        .fetch_all(db.pool())
        .await?;
    
    Ok(tokens)
}

/// Get tokens created by a specific address
pub async fn get_tokens_by_creator(db: &PostgresDB, creator_address: &str) -> Result<Vec<Token>> {
    let tokens = query_as::<_, Token>(
        r#"
        SELECT 
            t.contract_address,
            t.creator_address_id,
            t.is_scam,
            t.scam_label,
            t.creation_txn,
            t.trading_enabled_txn
        FROM eth_db.tokens t
        JOIN eth_db.addresses a ON t.creator_address_id = a.address_id
        WHERE a.address = $1
        "#
    )
    .bind(creator_address)
    .fetch_all(db.pool())
    .await?;
    
    Ok(tokens)
}

/// Get pools by type (V2, V3, V4)
pub async fn get_pools_by_type(
    db: &PostgresDB,
    pool_type: &str,
    limit: Option<i64>,
) -> Result<Vec<Pool>> {
    let mut sql = String::from(
        r#"
        SELECT 
            id,
            pool_address,
            pool_id,
            pool_type,
            token_address,
            pair_token_address,
            fee_tier,
            is_scam,
            scam_label,
            scam_block,
            scam_tx_hash,
            trading_enabled,
            trading_enabled_block,
            trading_enabled_txn
        FROM eth_db.pools
        WHERE pool_type = $1
        ORDER BY trading_enabled_block DESC
        "#
    );
    
    if let Some(lim) = limit {
        sql.push_str(&format!(" LIMIT {}", lim));
    }
    
    let pools = sqlx::query_as::<_, Pool>(&sql)
        .bind(pool_type)
        .fetch_all(db.pool())
        .await?;
    
    Ok(pools)
}

/// Get recently enabled trading pools
pub async fn get_recently_enabled_pools(
    db: &PostgresDB,
    blocks_back: i64,
    limit: Option<i64>,
) -> Result<Vec<Pool>> {
    // Estimate current block (roughly 20M as of 2024)
    let min_block = 20_000_000 - blocks_back;
    
    let mut sql = String::from(
        r#"
        SELECT 
            id,
            pool_address,
            pool_id,
            pool_type,
            token_address,
            pair_token_address,
            fee_tier,
            is_scam,
            scam_label,
            scam_block,
            scam_tx_hash,
            trading_enabled,
            trading_enabled_block,
            trading_enabled_txn
        FROM eth_db.pools
        WHERE trading_enabled = true 
        AND trading_enabled_block >= $1
        ORDER BY trading_enabled_block DESC
        "#
    );
    
    if let Some(lim) = limit {
        sql.push_str(&format!(" LIMIT {}", lim));
    }
    
    let pools = sqlx::query_as::<_, Pool>(&sql)
        .bind(min_block)
        .fetch_all(db.pool())
        .await?;
    
    Ok(pools)
}

/// Get scam token interactions for addresses
pub async fn get_scam_token_interactions(
    db: &PostgresDB,
    address: &str,
) -> Result<Vec<ScamTokenInteraction>> {
    use sqlx::{query, Row};
    
    let rows = query(
        r#"
        SELECT 
            tr.token_address,
            tok.scam_label,
            COUNT(*) as interaction_count,
            SUM(tr.total_denom_spent) as total_spent,
            SUM(tr.total_denom_received) as total_received,
            SUM(tr.realized_profit) as realized_profit
        FROM eth_db.trades tr
        JOIN eth_db.addresses a ON tr.address_id = a.address_id
        JOIN eth_db.tokens tok ON tr.token_address = tok.contract_address
        WHERE a.address = $1 AND tok.is_scam = true
        GROUP BY tr.token_address, tok.scam_label
        ORDER BY interaction_count DESC
        "#
    )
    .bind(address)
    .fetch_all(db.pool())
    .await?;
    
    let interactions = rows.into_iter().map(|row| ScamTokenInteraction {
        token_address: row.get("token_address"),
        scam_label: row.get("scam_label"),
        interaction_count: row.get("interaction_count"),
        total_spent: row.get("total_spent"),
        total_received: row.get("total_received"),
        realized_profit: row.get("realized_profit"),
    }).collect();
    
    Ok(interactions)
}

/// Get tokens ordered by pool count
pub async fn get_tokens_by_pool_count(
    db: &PostgresDB,
    limit: i64,
) -> Result<Vec<TokenWithPoolCount>> {
    use sqlx::{query, Row};
    
    let rows = query(
        r#"
        SELECT 
            t.contract_address,
            t.is_scam,
            t.scam_label,
            COUNT(p.id) as pool_count,
            COUNT(DISTINCT p.pool_type) as protocol_count,
            ARRAY_AGG(DISTINCT p.pool_type) as protocols
        FROM eth_db.tokens t
        LEFT JOIN eth_db.pools p ON t.contract_address = p.token_address
        GROUP BY t.contract_address, t.is_scam, t.scam_label
        ORDER BY pool_count DESC
        LIMIT $1
        "#
    )
    .bind(limit)
    .fetch_all(db.pool())
    .await?;
    
    let tokens = rows.into_iter().map(|row| TokenWithPoolCount {
        token_address: row.get("contract_address"),
        is_scam: row.get("is_scam"),
        scam_label: row.get("scam_label"),
        pool_count: row.get("pool_count"),
        protocol_count: row.get("protocol_count"),
        protocols: row.get("protocols"),
    }).collect();
    
    Ok(tokens)
}

/// Scam token interaction details
#[derive(Debug)]
pub struct ScamTokenInteraction {
    pub token_address: String,
    pub scam_label: Option<String>,
    pub interaction_count: i64,
    pub total_spent: Option<f64>,
    pub total_received: Option<f64>,
    pub realized_profit: Option<f64>,
}

/// Token with pool count
#[derive(Debug)]
pub struct TokenWithPoolCount {
    pub token_address: String,
    pub is_scam: Option<bool>,
    pub scam_label: Option<String>,
    pub pool_count: i64,
    pub protocol_count: i64,
    pub protocols: Option<Vec<String>>,
}