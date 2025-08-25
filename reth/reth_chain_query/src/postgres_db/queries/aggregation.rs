/// Aggregation Queries
/// 
/// Queries for aggregating trade data into address-level metrics.
/// Used to populate the addresses table from trades data.

use crate::postgres_db::connection::PostgresDB;
use eyre::Result;
use sqlx::{query, Row};

/// Aggregate all trade metrics into addresses table
pub async fn aggregate_address_metrics(db: &PostgresDB) -> Result<u64> {
    let result = query(
        r#"
        UPDATE eth_db.addresses a
        SET 
            total_profit = COALESCE(agg.total_profit, 0),
            total_volume = COALESCE(agg.total_volume, 0),
            total_realized_profit = COALESCE(agg.realized_profit, 0),
            total_erc20_trades = COALESCE(agg.trade_count, 0),
            total_tx_fee = COALESCE(agg.total_fees, 0),
            total_denom_balance = COALESCE(agg.denom_balance, 0),
            mean_received_spent_ratio = COALESCE(agg.avg_ratio, 0),
            first_seen = agg.first_block,
            last_seen = agg.last_block
        FROM (
            SELECT 
                address_id,
                SUM(realized_profit + unrealized_profit) as total_profit,
                SUM(total_denom_spent + total_denom_received) as total_volume,
                SUM(realized_profit) as realized_profit,
                COUNT(*) as trade_count,
                SUM(tx_fee) as total_fees,
                SUM(agg_denom_balance) as denom_balance,
                AVG(NULLIF(denom_received_spent_ratio, 0)) as avg_ratio,
                MIN(entry_block) as first_block,
                MAX(latest_block) as last_block
            FROM eth_db.trades
            GROUP BY address_id
        ) agg
        WHERE a.address_id = agg.address_id
        "#
    )
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Calculate and update scam ratios for all addresses
pub async fn calculate_scam_ratios(db: &PostgresDB) -> Result<u64> {
    let result = query(
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
            JOIN eth_db.tokens tok ON t.token_address = tok.contract_address
            GROUP BY t.address_id
        ) scam_stats
        WHERE a.address_id = scam_stats.address_id
        "#
    )
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Update address trade counts from trades table
pub async fn update_address_trade_counts(db: &PostgresDB) -> Result<u64> {
    let result = query(
        r#"
        UPDATE eth_db.addresses a
        SET 
            total_erc20_trades = trade_stats.total_trades,
            total_erc20_txn = trade_stats.total_transactions
        FROM (
            SELECT 
                address_id,
                COUNT(DISTINCT token_address) as total_trades,
                SUM(num_buys + num_sells) as total_transactions
            FROM eth_db.trades
            GROUP BY address_id
        ) trade_stats
        WHERE a.address_id = trade_stats.address_id
        "#
    )
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Batch update addresses with aggregated metrics (with progress tracking)
pub async fn batch_update_addresses(
    db: &PostgresDB,
    batch_size: i64,
    offset: i64,
) -> Result<u64> {
    let result = query(
        r#"
        WITH batch_addresses AS (
            SELECT DISTINCT address_id 
            FROM eth_db.trades
            ORDER BY address_id
            LIMIT $1 OFFSET $2
        )
        UPDATE eth_db.addresses a
        SET 
            total_profit = COALESCE(agg.total_profit, 0),
            total_volume = COALESCE(agg.total_volume, 0),
            total_realized_profit = COALESCE(agg.realized_profit, 0),
            total_erc20_trades = COALESCE(agg.trade_count, 0),
            total_tx_fee = COALESCE(agg.total_fees, 0)
        FROM (
            SELECT 
                t.address_id,
                SUM(t.realized_profit + t.unrealized_profit) as total_profit,
                SUM(t.total_denom_spent + t.total_denom_received) as total_volume,
                SUM(t.realized_profit) as realized_profit,
                COUNT(*) as trade_count,
                SUM(t.tx_fee) as total_fees
            FROM eth_db.trades t
            WHERE t.address_id IN (SELECT address_id FROM batch_addresses)
            GROUP BY t.address_id
        ) agg
        WHERE a.address_id = agg.address_id
        "#
    )
    .bind(batch_size)
    .bind(offset)
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Calculate profit metrics for addresses with recent trades
pub async fn calculate_profit_metrics(
    db: &PostgresDB,
    block_threshold: i32,
) -> Result<u64> {
    let result = query(
        r#"
        UPDATE eth_db.addresses a
        SET 
            total_profit = profit_calc.total_profit,
            total_realized_profit = profit_calc.realized_profit,
            mean_received_spent_ratio = profit_calc.mean_ratio,
            median_received_spent_ratio = profit_calc.median_ratio
        FROM (
            SELECT 
                address_id,
                SUM(realized_profit + unrealized_profit) as total_profit,
                SUM(realized_profit) as realized_profit,
                AVG(denom_received_spent_ratio) as mean_ratio,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY denom_received_spent_ratio) as median_ratio
            FROM eth_db.trades
            WHERE latest_block >= $1
            GROUP BY address_id
        ) profit_calc
        WHERE a.address_id = profit_calc.address_id
        "#
    )
    .bind(block_threshold)
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Update bribe amounts for addresses
pub async fn update_bribe_amounts(db: &PostgresDB) -> Result<u64> {
    let result = query(
        r#"
        UPDATE eth_db.addresses a
        SET 
            avg_bribe_amount = bribe_stats.avg_bribe,
            total_bribe_amount = bribe_stats.total_bribe
        FROM (
            SELECT 
                address_id,
                AVG(NULLIF(bribe_amount, 0)) as avg_bribe,
                SUM(bribe_amount) as total_bribe
            FROM eth_db.trades
            WHERE bribe_amount > 0
            GROUP BY address_id
        ) bribe_stats
        WHERE a.address_id = bribe_stats.address_id
        "#
    )
    .execute(db.pool())
    .await?;
    
    Ok(result.rows_affected())
}

/// Get aggregation progress statistics
pub async fn get_aggregation_progress(db: &PostgresDB) -> Result<AggregationProgress> {
    let row = query(
        r#"
        SELECT 
            (SELECT COUNT(DISTINCT address_id) FROM eth_db.trades) as addresses_with_trades,
            (SELECT COUNT(*) FROM eth_db.addresses WHERE total_profit IS NOT NULL) as addresses_aggregated,
            (SELECT COUNT(*) FROM eth_db.addresses WHERE scam_ratio IS NOT NULL) as addresses_with_scam_ratio,
            (SELECT COUNT(*) FROM eth_db.trades) as total_trades,
            (SELECT MAX(latest_block) FROM eth_db.trades) as latest_trade_block
        "#
    )
    .fetch_one(db.pool())
    .await?;
    
    Ok(AggregationProgress {
        addresses_with_trades: row.get("addresses_with_trades"),
        addresses_aggregated: row.get("addresses_aggregated"),
        addresses_with_scam_ratio: row.get("addresses_with_scam_ratio"),
        total_trades: row.get("total_trades"),
        latest_trade_block: row.get("latest_trade_block"),
    })
}

/// Progress tracking for aggregation
#[derive(Debug)]
pub struct AggregationProgress {
    pub addresses_with_trades: i64,
    pub addresses_aggregated: i64,
    pub addresses_with_scam_ratio: i64,
    pub total_trades: i64,
    pub latest_trade_block: Option<i32>,
}