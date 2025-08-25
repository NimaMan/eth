/// Trade Queries
/// 
/// Queries for trade-level data and PnL calculations for address-token pairs.

use crate::postgres_db::{connection::PostgresDB, models::{Trade, PnLAnalysis}};
use eyre::Result;
use sqlx::{query_as, query};

/// Get all trades for an address, optionally filtered by token
pub async fn get_trades_for_address(
    db: &PostgresDB,
    address: &str,
    token_address: Option<&str>,
) -> Result<Vec<Trade>> {
    let mut sql = String::from(
        r#"
        SELECT 
            t.id,
            t.address_id,
            t.token_address,
            t.currency,
            t.entry_block,
            t.latest_block,
            t.total_denom_spent,
            t.total_denom_received,
            t.denom_received_spent_ratio,
            t.bribe_amount,
            t.tx_fee,
            t.realized_profit,
            t.unrealized_profit,
            t.num_buys,
            t.num_sells,
            t.token_holdings_ratio,
            t.token_sell_buy_ratio,
            t.agg_denom_balance,
            t.agg_token_balance
        FROM eth_db.trades t
        JOIN eth_db.addresses a ON t.address_id = a.address_id
        WHERE a.address = $1
        "#
    );
    
    if token_address.is_some() {
        sql.push_str(" AND t.token_address = $2");
        
        let trades = sqlx::query_as::<_, Trade>(&sql)
            .bind(address)
            .bind(token_address.unwrap())
            .fetch_all(db.pool())
            .await?;
        
        Ok(trades)
    } else {
        let trades = sqlx::query_as::<_, Trade>(&sql)
            .bind(address)
            .fetch_all(db.pool())
            .await?;
        
        Ok(trades)
    }
}

/// Get PnL analysis for address-token pair
pub async fn get_address_token_pnl(
    db: &PostgresDB,
    address: &str,
    token_address: &str,
) -> Result<Option<PnLAnalysis>> {
    let result = query_as::<_, PnLAnalysis>(
        r#"
        SELECT 
            a.address,
            t.token_address,
            t.total_denom_spent as total_spent,
            t.total_denom_received as total_received,
            t.realized_profit,
            t.unrealized_profit,
            CASE 
                WHEN t.total_denom_spent > 0 THEN 
                    ((t.total_denom_received - t.total_denom_spent) / t.total_denom_spent) * 100
                ELSE 0
            END as roi_percentage,
            t.num_buys + t.num_sells as num_trades
        FROM eth_db.trades t
        JOIN eth_db.addresses a ON t.address_id = a.address_id
        WHERE a.address = $1 AND t.token_address = $2
        "#
    )
    .bind(address)
    .bind(token_address)
    .fetch_optional(db.pool())
    .await?;
    
    Ok(result)
}

/// Get most profitable trades across all addresses
pub async fn get_most_profitable_trades(
    db: &PostgresDB,
    limit: i64,
    min_volume: Option<f64>,
) -> Result<Vec<Trade>> {
    let mut sql = String::from(
        r#"
        SELECT 
            t.id,
            t.address_id,
            t.token_address,
            t.currency,
            t.entry_block,
            t.latest_block,
            t.total_denom_spent,
            t.total_denom_received,
            t.denom_received_spent_ratio,
            t.bribe_amount,
            t.tx_fee,
            t.realized_profit,
            t.unrealized_profit,
            t.num_buys,
            t.num_sells,
            t.token_holdings_ratio,
            t.token_sell_buy_ratio,
            t.agg_denom_balance,
            t.agg_token_balance
        FROM eth_db.trades t
        WHERE t.realized_profit IS NOT NULL
        "#
    );
    
    if let Some(min_vol) = min_volume {
        sql.push_str(&format!(" AND (t.total_denom_spent + t.total_denom_received) >= {}", min_vol));
    }
    
    sql.push_str(" ORDER BY t.realized_profit DESC");
    sql.push_str(&format!(" LIMIT {}", limit));
    
    let trades = sqlx::query_as::<_, Trade>(&sql)
        .fetch_all(db.pool())
        .await?;
    
    Ok(trades)
}

/// Get recent trades within block range
pub async fn get_recent_trades(
    db: &PostgresDB,
    start_block: i32,
    end_block: i32,
    limit: Option<i64>,
) -> Result<Vec<Trade>> {
    let mut sql = String::from(
        r#"
        SELECT 
            t.id,
            t.address_id,
            t.token_address,
            t.currency,
            t.entry_block,
            t.latest_block,
            t.total_denom_spent,
            t.total_denom_received,
            t.denom_received_spent_ratio,
            t.bribe_amount,
            t.tx_fee,
            t.realized_profit,
            t.unrealized_profit,
            t.num_buys,
            t.num_sells,
            t.token_holdings_ratio,
            t.token_sell_buy_ratio,
            t.agg_denom_balance,
            t.agg_token_balance
        FROM eth_db.trades t
        WHERE t.latest_block >= $1 AND t.latest_block <= $2
        ORDER BY t.latest_block DESC
        "#
    );
    
    if let Some(lim) = limit {
        sql.push_str(&format!(" LIMIT {}", lim));
    }
    
    let trades = sqlx::query_as::<_, Trade>(&sql)
        .bind(start_block)
        .bind(end_block)
        .fetch_all(db.pool())
        .await?;
    
    Ok(trades)
}

/// Get trades for batch aggregation
pub async fn get_trades_for_aggregation(
    db: &PostgresDB,
    address_ids: &[i64],
) -> Result<Vec<Trade>> {
    let trades = query_as::<_, Trade>(
        r#"
        SELECT 
            t.id,
            t.address_id,
            t.token_address,
            t.currency,
            t.entry_block,
            t.latest_block,
            t.total_denom_spent,
            t.total_denom_received,
            t.denom_received_spent_ratio,
            t.bribe_amount,
            t.tx_fee,
            t.realized_profit,
            t.unrealized_profit,
            t.num_buys,
            t.num_sells,
            t.token_holdings_ratio,
            t.token_sell_buy_ratio,
            t.agg_denom_balance,
            t.agg_token_balance
        FROM eth_db.trades t
        WHERE t.address_id = ANY($1)
        ORDER BY t.address_id, t.latest_block DESC
        "#
    )
    .bind(address_ids)
    .fetch_all(db.pool())
    .await?;
    
    Ok(trades)
}

/// Get summarized trade metrics per address
pub async fn get_address_trade_summary(
    db: &PostgresDB,
    address: &str,
) -> Result<Option<TradeSummary>> {
    use sqlx::Row;
    
    let row = query(
        r#"
        SELECT 
            a.address,
            COUNT(DISTINCT t.token_address) as unique_tokens,
            COUNT(t.id) as total_trades,
            SUM(t.num_buys) as total_buys,
            SUM(t.num_sells) as total_sells,
            SUM(t.total_denom_spent) as total_spent,
            SUM(t.total_denom_received) as total_received,
            SUM(t.realized_profit) as total_realized_profit,
            SUM(t.unrealized_profit) as total_unrealized_profit,
            SUM(t.tx_fee) as total_fees,
            MIN(t.entry_block) as first_trade_block,
            MAX(t.latest_block) as last_trade_block
        FROM eth_db.trades t
        JOIN eth_db.addresses a ON t.address_id = a.address_id
        WHERE a.address = $1
        GROUP BY a.address
        "#
    )
    .bind(address)
    .fetch_optional(db.pool())
    .await?;
    
    match row {
        Some(r) => Ok(Some(TradeSummary {
            address: r.get("address"),
            unique_tokens: r.get("unique_tokens"),
            total_trades: r.get("total_trades"),
            total_buys: r.get("total_buys"),
            total_sells: r.get("total_sells"),
            total_spent: r.get("total_spent"),
            total_received: r.get("total_received"),
            total_realized_profit: r.get("total_realized_profit"),
            total_unrealized_profit: r.get("total_unrealized_profit"),
            total_fees: r.get("total_fees"),
            first_trade_block: r.get("first_trade_block"),
            last_trade_block: r.get("last_trade_block"),
        })),
        None => Ok(None),
    }
}

/// Trade summary structure
#[derive(Debug)]
pub struct TradeSummary {
    pub address: String,
    pub unique_tokens: i64,
    pub total_trades: i64,
    pub total_buys: Option<i64>,
    pub total_sells: Option<i64>,
    pub total_spent: Option<f64>,
    pub total_received: Option<f64>,
    pub total_realized_profit: Option<f64>,
    pub total_unrealized_profit: Option<f64>,
    pub total_fees: Option<f64>,
    pub first_trade_block: Option<i32>,
    pub last_trade_block: Option<i32>,
}