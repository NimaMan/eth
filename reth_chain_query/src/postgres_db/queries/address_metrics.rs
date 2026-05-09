/// Address Metrics Queries
///
/// Queries for address-level aggregated metrics including PnL, volume, and activity.
use crate::postgres_db::{connection::PostgresDB, models::AddressMetrics};
use eyre::Result;
use sqlx::query_as;

/// Get aggregated metrics for a specific address
pub async fn get_address_metrics(db: &PostgresDB, address: &str) -> Result<Option<AddressMetrics>> {
    let metrics = query_as::<_, AddressMetrics>(
        r#"
        SELECT 
            address_id,
            address,
            is_contract,
            total_erc20_tx,
            total_erc20_trades,
            scam_ratio,
            total_profit,
            total_volume,
            first_seen,
            last_seen,
            total_tx_fee,
            degree_centrality,
            betweenness_centrality,
            total_denom_balance,
            total_realized_profit,
            mean_received_spent_ratio,
            median_received_spent_ratio,
            avg_bribe_amount,
            total_bribe_amount,
            name,
            entity_category,
            cluster_label
        FROM eth_db.addresses
        WHERE address = $1
        "#,
    )
    .bind(address)
    .fetch_optional(db.pool())
    .await?;

    Ok(metrics)
}

/// Get top profitable addresses with filters
pub async fn get_top_profitable_addresses(
    db: &PostgresDB,
    limit: i64,
    min_volume: Option<f64>,
    exclude_contracts: bool,
) -> Result<Vec<AddressMetrics>> {
    let mut query = String::from(
        r#"
        SELECT 
            address_id,
            address,
            is_contract,
            total_erc20_tx,
            total_erc20_trades,
            scam_ratio,
            total_profit,
            total_volume,
            first_seen,
            last_seen,
            total_tx_fee,
            degree_centrality,
            betweenness_centrality,
            total_denom_balance,
            total_realized_profit,
            mean_received_spent_ratio,
            median_received_spent_ratio,
            avg_bribe_amount,
            total_bribe_amount,
            name,
            entity_category,
            cluster_label
        FROM eth_db.addresses
        WHERE 1=1
        "#,
    );

    if let Some(min_vol) = min_volume {
        query.push_str(&format!(" AND total_volume >= {}", min_vol));
    }

    if exclude_contracts {
        query.push_str(" AND is_contract = false");
    }

    query.push_str(" ORDER BY total_profit DESC NULLS LAST");
    query.push_str(&format!(" LIMIT {}", limit));

    let addresses = sqlx::query_as::<_, AddressMetrics>(&query)
        .fetch_all(db.pool())
        .await?;

    Ok(addresses)
}

/// Get addresses by scam ratio threshold
pub async fn get_high_scam_ratio_addresses(
    db: &PostgresDB,
    min_scam_ratio: f64,
    limit: i64,
) -> Result<Vec<AddressMetrics>> {
    let addresses = query_as::<_, AddressMetrics>(
        r#"
        SELECT 
            address_id,
            address,
            is_contract,
            total_erc20_tx,
            total_erc20_trades,
            scam_ratio,
            total_profit,
            total_volume,
            first_seen,
            last_seen,
            total_tx_fee,
            degree_centrality,
            betweenness_centrality,
            total_denom_balance,
            total_realized_profit,
            mean_received_spent_ratio,
            median_received_spent_ratio,
            avg_bribe_amount,
            total_bribe_amount,
            name,
            entity_category,
            cluster_label
        FROM eth_db.addresses
        WHERE scam_ratio >= $1
        ORDER BY scam_ratio DESC
        LIMIT $2
        "#,
    )
    .bind(min_scam_ratio)
    .bind(limit)
    .fetch_all(db.pool())
    .await?;

    Ok(addresses)
}

/// Get most active addresses by trade count
pub async fn get_most_active_addresses(
    db: &PostgresDB,
    limit: i64,
    time_window_blocks: Option<i32>,
) -> Result<Vec<AddressMetrics>> {
    let mut query = String::from(
        r#"
        SELECT 
            address_id,
            address,
            is_contract,
            total_erc20_tx,
            total_erc20_trades,
            scam_ratio,
            total_profit,
            total_volume,
            first_seen,
            last_seen,
            total_tx_fee,
            degree_centrality,
            betweenness_centrality,
            total_denom_balance,
            total_realized_profit,
            mean_received_spent_ratio,
            median_received_spent_ratio,
            avg_bribe_amount,
            total_bribe_amount,
            name,
            entity_category,
            cluster_label
        FROM eth_db.addresses
        WHERE total_erc20_trades IS NOT NULL
        "#,
    );

    if let Some(window) = time_window_blocks {
        // Get current block estimate (roughly 20M as of 2024)
        query.push_str(&format!(" AND last_seen >= (20000000 - {})", window));
    }

    query.push_str(" ORDER BY total_erc20_trades DESC");
    query.push_str(&format!(" LIMIT {}", limit));

    let addresses = sqlx::query_as::<_, AddressMetrics>(&query)
        .fetch_all(db.pool())
        .await?;

    Ok(addresses)
}
