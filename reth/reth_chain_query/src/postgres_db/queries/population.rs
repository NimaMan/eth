/// Population Helper Queries
/// 
/// Helper queries for populating and maintaining database integrity.
/// Includes functions for identifying missing data and incremental updates.

use crate::postgres_db::connection::PostgresDB;
use eyre::Result;
use sqlx::{query, query_as, Row};
use serde::{Deserialize, Serialize};

/// Main function to populate addresses from trades
pub async fn populate_addresses_from_trades(
    db: &PostgresDB,
    full_refresh: bool,
) -> Result<PopulationResult> {
    // Start transaction
    let mut tx = db.pool().begin().await?;
    
    // Clear existing metrics if full refresh
    if full_refresh {
        query("UPDATE eth_db.addresses SET 
            total_profit = NULL,
            total_volume = NULL,
            total_realized_profit = NULL,
            total_erc20_trades = NULL,
            scam_ratio = NULL,
            total_tx_fee = NULL
        ")
        .execute(&mut *tx)
        .await?;
    }
    
    // Aggregate metrics from trades
    let profit_rows = query(
        r#"
        UPDATE eth_db.addresses a
        SET 
            total_profit = agg.total_profit,
            total_volume = agg.total_volume,
            total_realized_profit = agg.realized_profit,
            total_erc20_trades = agg.trade_count,
            total_tx_fee = agg.total_fees,
            total_denom_balance = agg.denom_balance,
            mean_received_spent_ratio = agg.avg_ratio,
            first_seen = LEAST(a.first_seen, agg.first_block),
            last_seen = GREATEST(a.last_seen, agg.last_block)
        FROM (
            SELECT 
                address_id,
                SUM(COALESCE(realized_profit, 0) + COALESCE(unrealized_profit, 0)) as total_profit,
                SUM(COALESCE(total_denom_spent, 0) + COALESCE(total_denom_received, 0)) as total_volume,
                SUM(COALESCE(realized_profit, 0)) as realized_profit,
                COUNT(*) as trade_count,
                SUM(COALESCE(tx_fee, 0)) as total_fees,
                SUM(COALESCE(agg_denom_balance, 0)) as denom_balance,
                AVG(NULLIF(denom_received_spent_ratio, 0)) as avg_ratio,
                MIN(entry_block) as first_block,
                MAX(latest_block) as last_block
            FROM eth_db.trades
            GROUP BY address_id
        ) agg
        WHERE a.address_id = agg.address_id
        "#
    )
    .execute(&mut *tx)
    .await?;
    
    // Calculate scam ratios
    let scam_rows = query(
        r#"
        UPDATE eth_db.addresses a
        SET scam_ratio = COALESCE(scam_stats.scam_ratio, 0)
        FROM (
            SELECT 
                t.address_id,
                CASE 
                    WHEN COUNT(*) = 0 THEN 0
                    ELSE COUNT(*) FILTER (WHERE tok.is_scam = true)::float / COUNT(*)::float
                END as scam_ratio
            FROM eth_db.trades t
            LEFT JOIN eth_db.tokens tok ON t.token_address = tok.contract_address
            GROUP BY t.address_id
        ) scam_stats
        WHERE a.address_id = scam_stats.address_id
        "#
    )
    .execute(&mut *tx)
    .await?;
    
    // Commit transaction
    tx.commit().await?;
    
    Ok(PopulationResult {
        addresses_updated_profit: profit_rows.rows_affected(),
        addresses_updated_scam: scam_rows.rows_affected(),
    })
}

/// Find addresses that need metric updates
pub async fn get_addresses_needing_update(
    db: &PostgresDB,
    limit: i64,
) -> Result<Vec<AddressForUpdate>> {
    let addresses = query_as::<_, AddressForUpdate>(
        r#"
        SELECT DISTINCT 
            a.address_id,
            a.address,
            COUNT(t.id) as trade_count,
            MAX(t.latest_block) as latest_trade_block,
            a.last_seen as last_updated_block
        FROM eth_db.addresses a
        JOIN eth_db.trades t ON a.address_id = t.address_id
        WHERE 
            a.total_profit IS NULL 
            OR a.last_seen < t.latest_block
            OR a.total_erc20_trades != COUNT(t.id)
        GROUP BY a.address_id, a.address, a.last_seen
        ORDER BY COUNT(t.id) DESC
        LIMIT $1
        "#
    )
    .bind(limit)
    .fetch_all(db.pool())
    .await?;
    
    Ok(addresses)
}

/// Calculate profit metrics for specific addresses
pub async fn calculate_profit_metrics_for_addresses(
    db: &PostgresDB,
    address_ids: &[i64],
) -> Result<u64> {
    let result = query(
        r#"
        UPDATE eth_db.addresses a
        SET 
            total_profit = profit_calc.total_profit,
            total_realized_profit = profit_calc.realized_profit,
            mean_received_spent_ratio = profit_calc.mean_ratio,
            median_received_spent_ratio = profit_calc.median_ratio,
            total_denom_balance = profit_calc.balance
        FROM (
            SELECT 
                address_id,
                SUM(COALESCE(realized_profit, 0) + COALESCE(unrealized_profit, 0)) as total_profit,
                SUM(COALESCE(realized_profit, 0)) as realized_profit,
                AVG(NULLIF(denom_received_spent_ratio, 0)) as mean_ratio,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY NULLIF(denom_received_spent_ratio, 0)) as median_ratio,
                SUM(COALESCE(agg_denom_balance, 0)) as balance
            FROM eth_db.trades
            WHERE address_id = ANY($1)
            GROUP BY address_id
        ) profit_calc
        WHERE a.address_id = profit_calc.address_id
        "#
    )
    .bind(address_ids)
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Update address balances from latest trades
pub async fn update_address_balances(db: &PostgresDB) -> Result<u64> {
    let result = query(
        r#"
        UPDATE eth_db.addresses a
        SET total_denom_balance = balance_calc.latest_balance
        FROM (
            SELECT DISTINCT ON (address_id)
                address_id,
                agg_denom_balance as latest_balance
            FROM eth_db.trades
            ORDER BY address_id, latest_block DESC
        ) balance_calc
        WHERE a.address_id = balance_calc.address_id
        "#
    )
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Ensure all addresses from trades exist in addresses table
pub async fn ensure_addresses_exist(db: &PostgresDB) -> Result<u64> {
    let result = query(
        r#"
        INSERT INTO eth_db.addresses (address_id, address, is_contract)
        SELECT DISTINCT 
            t.address_id,
            COALESCE(
                (SELECT address FROM eth_db.addresses WHERE address_id = t.address_id LIMIT 1),
                '0x' || LPAD(t.address_id::text, 40, '0')
            ) as address,
            false as is_contract
        FROM eth_db.trades t
        LEFT JOIN eth_db.addresses a ON t.address_id = a.address_id
        WHERE a.address_id IS NULL
        ON CONFLICT (address_id) DO NOTHING
        "#
    )
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Get statistics about data population status
pub async fn get_population_statistics(db: &PostgresDB) -> Result<PopulationStats> {
    let row = query(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM eth_db.addresses) as total_addresses,
            (SELECT COUNT(*) FROM eth_db.addresses WHERE total_profit IS NOT NULL) as addresses_with_profit,
            (SELECT COUNT(*) FROM eth_db.addresses WHERE scam_ratio IS NOT NULL) as addresses_with_scam_ratio,
            (SELECT COUNT(*) FROM eth_db.addresses WHERE total_erc20_trades > 0) as addresses_with_trades,
            (SELECT COUNT(DISTINCT address_id) FROM eth_db.trades) as unique_trading_addresses,
            (SELECT COUNT(*) FROM eth_db.trades) as total_trades,
            (SELECT COUNT(*) FROM eth_db.tokens) as total_tokens,
            (SELECT COUNT(*) FROM eth_db.tokens WHERE is_scam = true) as scam_tokens,
            (SELECT COUNT(*) FROM eth_db.pools) as total_pools
        "#
    )
    .fetch_one(db.pool())
    .await?;
    
    Ok(PopulationStats {
        total_addresses: row.get("total_addresses"),
        addresses_with_profit: row.get("addresses_with_profit"),
        addresses_with_scam_ratio: row.get("addresses_with_scam_ratio"),
        addresses_with_trades: row.get("addresses_with_trades"),
        unique_trading_addresses: row.get("unique_trading_addresses"),
        total_trades: row.get("total_trades"),
        total_tokens: row.get("total_tokens"),
        scam_tokens: row.get("scam_tokens"),
        total_pools: row.get("total_pools"),
    })
}

/// Result of population operation
#[derive(Debug, Serialize, Deserialize)]
pub struct PopulationResult {
    pub addresses_updated_profit: u64,
    pub addresses_updated_scam: u64,
}

/// Address needing update
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AddressForUpdate {
    pub address_id: i64,
    pub address: String,
    pub trade_count: i64,
    pub latest_trade_block: Option<i32>,
    pub last_updated_block: Option<i32>,
}

/// Population statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct PopulationStats {
    pub total_addresses: i64,
    pub addresses_with_profit: i64,
    pub addresses_with_scam_ratio: i64,
    pub addresses_with_trades: i64,
    pub unique_trading_addresses: i64,
    pub total_trades: i64,
    pub total_tokens: i64,
    pub scam_tokens: i64,
    pub total_pools: i64,
}