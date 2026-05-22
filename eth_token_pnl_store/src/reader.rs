use sqlx::PgPool;

use crate::{
    schema::{AddressPnlRow, PnlMovementRow, PoolPnlStateRow},
    Result,
};

#[derive(Clone)]
pub struct TokenPnlReader {
    pool: PgPool,
}

impl TokenPnlReader {
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn pool_state(&self, run_id: &str, pool_id: &str) -> Result<Option<PoolPnlStateRow>> {
        let row = sqlx::query_as::<_, PoolPnlStateRow>(
            r#"
            SELECT
                run_id, pool_id, token_address, denom_address, protocol,
                token_decimals, denom_decimals, tx_count, latest_block, latest_timestamp,
                token_in_raw::text AS token_in_raw,
                token_out_raw::text AS token_out_raw,
                denom_in_raw::text AS denom_in_raw,
                denom_out_raw::text AS denom_out_raw,
                pool_token_in_raw::text AS pool_token_in_raw,
                pool_token_out_raw::text AS pool_token_out_raw,
                pool_denom_in_raw::text AS pool_denom_in_raw,
                pool_denom_out_raw::text AS pool_denom_out_raw,
                native_fee_raw::text AS native_fee_raw,
                native_bribe_raw::text AS native_bribe_raw,
                token_transfer_count,
                denom_transfer_count
            FROM token_pnl.pool_pnl_states
            WHERE run_id = $1 AND pool_id = $2
            "#,
        )
        .bind(run_id)
        .bind(pool_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn top_addresses_by_denom_cashflow(
        &self,
        run_id: &str,
        pool_id: &str,
        limit: i64,
    ) -> Result<Vec<AddressPnlRow>> {
        let rows = sqlx::query_as::<_, AddressPnlRow>(
            r#"
            SELECT
                run_id, pool_id, address,
                token_in_raw::text AS token_in_raw,
                token_out_raw::text AS token_out_raw,
                denom_in_raw::text AS denom_in_raw,
                denom_out_raw::text AS denom_out_raw,
                native_fee_raw::text AS native_fee_raw,
                native_bribe_raw::text AS native_bribe_raw,
                token_balance_raw,
                denom_cashflow_raw,
                token_balance,
                denom_cashflow,
                native_fee,
                native_bribe,
                marked_token_value_denom,
                pnl_proxy_denom,
                first_block,
                latest_block,
                movement_count
            FROM token_pnl.pool_address_pnl
            WHERE run_id = $1 AND pool_id = $2
            ORDER BY abs(denom_cashflow) DESC NULLS LAST, movement_count DESC
            LIMIT $3
            "#,
        )
        .bind(run_id)
        .bind(pool_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn movements_for_address(
        &self,
        run_id: &str,
        pool_id: &str,
        address: &str,
        limit: i64,
    ) -> Result<Vec<PnlMovementRow>> {
        let rows = sqlx::query_as::<_, PnlMovementRow>(
            r#"
            SELECT
                run_id, pool_id, entry_index, tx_hash, block_number, block_timestamp,
                tx_index, log_index, address, kind,
                token_in_raw::text AS token_in_raw,
                token_out_raw::text AS token_out_raw,
                denom_in_raw::text AS denom_in_raw,
                denom_out_raw::text AS denom_out_raw,
                native_fee_raw::text AS native_fee_raw,
                native_bribe_raw::text AS native_bribe_raw,
                pool_direct
            FROM token_pnl.pool_pnl_movements
            WHERE run_id = $1 AND pool_id = $2 AND address = $3
            ORDER BY entry_index
            LIMIT $4
            "#,
        )
        .bind(run_id)
        .bind(pool_id)
        .bind(address)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
