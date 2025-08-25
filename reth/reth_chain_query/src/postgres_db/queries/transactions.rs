/// Transaction Queries
/// 
/// Queries for raw transactions and participants, used for building transaction graphs
/// and fund flow analysis. These queries work with the tx_participants and transactions tables.

use crate::postgres_db::{connection::PostgresDB, models::{Transaction, TxParticipant, AddressTransaction, TransactionWithParticipants}};
use eyre::Result;
use sqlx::{query, query_as, Row};

/// Get address_id from address string (checksum format)
pub async fn get_address_id(
    db: &PostgresDB,
    address: &str,
) -> Result<Option<i64>> {
    let id = sqlx::query_scalar::<_, i64>(
        "SELECT address_id FROM eth_db.addresses WHERE address = $1"
    )
    .bind(address)
    .fetch_optional(db.pool())
    .await?;
    
    Ok(id)
}

/// Get all transactions where an address is a participant
/// Returns transactions with other participants for graph building
pub async fn get_address_transactions(
    db: &PostgresDB,
    address: &str,
    limit: usize,
    max_block: Option<i32>,
) -> Result<Vec<AddressTransaction>> {
    // Get address_id first
    let address_id = get_address_id(db, address).await?
        .ok_or_else(|| eyre::eyre!("Address not found: {}", address))?;
    
    let sql = if let Some(max_block) = max_block {
        r#"
        SELECT DISTINCT
            t.tx_hash,
            t.block_number,
            a_from.address as from_address,
            a_to.address as to_address,
            t.value,
            (SELECT COUNT(DISTINCT address_id) FROM eth_db.tx_participants WHERE tx_hash = t.tx_hash) as participant_count
        FROM eth_db.tx_participants tp
        JOIN eth_db.transactions t ON t.tx_hash = tp.tx_hash
        LEFT JOIN eth_db.addresses a_from ON t.from_address_id = a_from.address_id
        LEFT JOIN eth_db.addresses a_to ON t.to_address_id = a_to.address_id
        WHERE tp.address_id = $1 AND t.block_number <= $2
        ORDER BY t.block_number DESC
        LIMIT $3
        "#
    } else {
        r#"
        SELECT DISTINCT
            t.tx_hash,
            t.block_number,
            a_from.address as from_address,
            a_to.address as to_address,
            t.value,
            (SELECT COUNT(DISTINCT address_id) FROM eth_db.tx_participants WHERE tx_hash = t.tx_hash) as participant_count
        FROM eth_db.tx_participants tp
        JOIN eth_db.transactions t ON t.tx_hash = tp.tx_hash
        LEFT JOIN eth_db.addresses a_from ON t.from_address_id = a_from.address_id
        LEFT JOIN eth_db.addresses a_to ON t.to_address_id = a_to.address_id
        WHERE tp.address_id = $1
        ORDER BY t.block_number DESC
        LIMIT $2
        "#
    };
    
    let rows = if let Some(max_block) = max_block {
        query(sql)
            .bind(address_id)
            .bind(max_block)
            .bind(limit as i64)
            .fetch_all(db.pool())
            .await?
    } else {
        query(sql)
            .bind(address_id)
            .bind(limit as i64)
            .fetch_all(db.pool())
            .await?
    };
    
    let transactions = rows.into_iter().map(|row| AddressTransaction {
        tx_hash: row.get("tx_hash"),
        block_number: row.get("block_number"),
        from_address: row.get("from_address"),
        to_address: row.get("to_address"),
        value: row.get("value"),
        participant_count: row.get("participant_count"),
    }).collect();
    
    Ok(transactions)
}

/// Get all participants in a transaction (addresses involved)
pub async fn get_tx_participants(
    db: &PostgresDB,
    tx_hash: &str,
) -> Result<Vec<String>> {
    let rows = query(
        r#"
        SELECT DISTINCT a.address
        FROM eth_db.tx_participants tp
        JOIN eth_db.addresses a ON tp.address_id = a.address_id
        WHERE tp.tx_hash = $1
        "#
    )
    .bind(tx_hash)
    .fetch_all(db.pool())
    .await?;
    
    let participants = rows.into_iter()
        .map(|row| row.get("address"))
        .collect();
    
    Ok(participants)
}

/// Get all participant address_ids in a transaction
pub async fn get_tx_participant_ids(
    db: &PostgresDB,
    tx_hash: &str,
) -> Result<Vec<i64>> {
    let ids = query_as::<_, (i64,)>(
        "SELECT DISTINCT address_id FROM eth_db.tx_participants WHERE tx_hash = $1"
    )
    .bind(tx_hash)
    .fetch_all(db.pool())
    .await?
    .into_iter()
    .map(|(id,)| id)
    .collect();
    
    Ok(ids)
}

/// Get transaction details with all participants
pub async fn get_transaction_with_participants(
    db: &PostgresDB,
    tx_hash: &str,
) -> Result<Option<TransactionWithParticipants>> {
    // Get transaction details
    let tx_row = query(
        r#"
        SELECT 
            t.tx_hash,
            t.block_number,
            a_from.address as from_address,
            a_to.address as to_address,
            t.value,
            t.status
        FROM eth_db.transactions t
        LEFT JOIN eth_db.addresses a_from ON t.from_address_id = a_from.address_id
        LEFT JOIN eth_db.addresses a_to ON t.to_address_id = a_to.address_id
        WHERE t.tx_hash = $1
        "#
    )
    .bind(tx_hash)
    .fetch_optional(db.pool())
    .await?;
    
    match tx_row {
        Some(row) => {
            // Get all participants
            let participants = get_tx_participants(db, tx_hash).await?;
            
            Ok(Some(TransactionWithParticipants {
                tx_hash: row.get("tx_hash"),
                block_number: row.get("block_number"),
                from_address: row.get("from_address"),
                to_address: row.get("to_address"),
                value: row.get("value"),
                status: row.get("status"),
                participants,
            }))
        },
        None => Ok(None),
    }
}

/// Find transactions between two addresses
pub async fn get_transactions_between_addresses(
    db: &PostgresDB,
    address1: &str,
    address2: &str,
    limit: usize,
) -> Result<Vec<Transaction>> {
    // Get address_ids
    let addr1_id = get_address_id(db, address1).await?
        .ok_or_else(|| eyre::eyre!("Address not found: {}", address1))?;
    let addr2_id = get_address_id(db, address2).await?
        .ok_or_else(|| eyre::eyre!("Address not found: {}", address2))?;
    
    let transactions = query_as::<_, Transaction>(
        r#"
        SELECT DISTINCT t.*
        FROM eth_db.transactions t
        WHERE EXISTS (
            SELECT 1 FROM eth_db.tx_participants tp1
            WHERE tp1.tx_hash = t.tx_hash AND tp1.address_id = $1
        )
        AND EXISTS (
            SELECT 1 FROM eth_db.tx_participants tp2
            WHERE tp2.tx_hash = t.tx_hash AND tp2.address_id = $2
        )
        ORDER BY t.block_number DESC
        LIMIT $3
        "#
    )
    .bind(addr1_id)
    .bind(addr2_id)
    .bind(limit as i64)
    .fetch_all(db.pool())
    .await?;
    
    Ok(transactions)
}

/// Get recent transactions (for monitoring)
pub async fn get_recent_transactions(
    db: &PostgresDB,
    start_block: i32,
    limit: usize,
) -> Result<Vec<Transaction>> {
    let transactions = query_as::<_, Transaction>(
        r#"
        SELECT 
            tx_hash,
            block_number,
            from_address_id,
            to_address_id,
            value,
            status
        FROM eth_db.transactions
        WHERE block_number >= $1
        ORDER BY block_number DESC
        LIMIT $2
        "#
    )
    .bind(start_block)
    .bind(limit as i64)
    .fetch_all(db.pool())
    .await?;
    
    Ok(transactions)
}

/// Count transactions for an address
pub async fn count_address_transactions(
    db: &PostgresDB,
    address: &str,
) -> Result<i64> {
    let address_id = get_address_id(db, address).await?
        .ok_or_else(|| eyre::eyre!("Address not found: {}", address))?;
    
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT tx_hash) FROM eth_db.tx_participants WHERE address_id = $1"
    )
    .bind(address_id)
    .fetch_one(db.pool())
    .await?;
    
    Ok(count)
}

/// Get transactions for multiple addresses (batch operation)
pub async fn get_batch_address_transactions(
    db: &PostgresDB,
    addresses: &[String],
    limit_per_address: usize,
) -> Result<Vec<AddressTransaction>> {
    // Get address_ids
    let mut address_ids = Vec::new();
    for addr in addresses {
        if let Some(id) = get_address_id(db, addr).await? {
            address_ids.push(id);
        }
    }
    
    if address_ids.is_empty() {
        return Ok(Vec::new());
    }
    
    let rows = query(
        r#"
        WITH ranked_txs AS (
            SELECT 
                t.tx_hash,
                t.block_number,
                a_from.address as from_address,
                a_to.address as to_address,
                t.value,
                tp.address_id,
                ROW_NUMBER() OVER (PARTITION BY tp.address_id ORDER BY t.block_number DESC) as rn
            FROM eth_db.tx_participants tp
            JOIN eth_db.transactions t ON t.tx_hash = tp.tx_hash
            LEFT JOIN eth_db.addresses a_from ON t.from_address_id = a_from.address_id
            LEFT JOIN eth_db.addresses a_to ON t.to_address_id = a_to.address_id
            WHERE tp.address_id = ANY($1)
        )
        SELECT 
            tx_hash,
            block_number,
            from_address,
            to_address,
            value,
            (SELECT COUNT(DISTINCT address_id) FROM eth_db.tx_participants WHERE tx_hash = ranked_txs.tx_hash) as participant_count
        FROM ranked_txs
        WHERE rn <= $2
        ORDER BY block_number DESC
        "#
    )
    .bind(&address_ids)
    .bind(limit_per_address as i64)
    .fetch_all(db.pool())
    .await?;
    
    let transactions = rows.into_iter().map(|row| AddressTransaction {
        tx_hash: row.get("tx_hash"),
        block_number: row.get("block_number"),
        from_address: row.get("from_address"),
        to_address: row.get("to_address"),
        value: row.get("value"),
        participant_count: row.get("participant_count"),
    }).collect();
    
    Ok(transactions)
}