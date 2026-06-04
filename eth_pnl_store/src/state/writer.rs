use sqlx::PgPool;

use super::{
    error::{optional_u64_to_i64, u64_to_i64},
    migration::run_migrations,
    schema::{PoolLatestState, TokenLatestState},
    Result, TokenStateStoreConfig,
};

#[derive(Clone)]
pub struct TokenStateStore {
    pool: PgPool,
}

impl TokenStateStore {
    pub async fn connect(config: &TokenStateStoreConfig) -> Result<Self> {
        let pool = PgPool::connect(&config.database_url).await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn run_migrations(&self) -> Result<()> {
        run_migrations(&self.pool).await
    }

    pub async fn delete_scope(&self, scope_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM token_state.token_latest WHERE scope_id = $1")
            .bind(scope_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn write_token_latest(&self, state: &TokenLatestState) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO token_state.token_latest (
                scope_id, chain_id, token_address, source_run_id, as_of_block, source_reason,
                name, symbol, decimals, total_supply_raw,
                creation_block, creation_timestamp, creation_tx, creator_address,
                latest_activity_block, latest_activity_timestamp, latest_pool_activity_block,
                lifecycle_status, is_scam, scam_label, scam_mechanism, scam_mechanism_label,
                pool_count, active_pool_count, retained_pool_count, liquidity_removal_pool_count
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10,
                $11, $12, $13, $14,
                $15, $16, $17,
                $18, $19, $20, $21, $22,
                $23, $24, $25, $26
            )
            ON CONFLICT (scope_id, chain_id, token_address) DO UPDATE SET
                source_run_id = EXCLUDED.source_run_id,
                as_of_block = EXCLUDED.as_of_block,
                source_reason = EXCLUDED.source_reason,
                name = EXCLUDED.name,
                symbol = EXCLUDED.symbol,
                decimals = EXCLUDED.decimals,
                total_supply_raw = EXCLUDED.total_supply_raw,
                creation_block = EXCLUDED.creation_block,
                creation_timestamp = EXCLUDED.creation_timestamp,
                creation_tx = EXCLUDED.creation_tx,
                creator_address = EXCLUDED.creator_address,
                latest_activity_block = EXCLUDED.latest_activity_block,
                latest_activity_timestamp = EXCLUDED.latest_activity_timestamp,
                latest_pool_activity_block = EXCLUDED.latest_pool_activity_block,
                lifecycle_status = EXCLUDED.lifecycle_status,
                is_scam = EXCLUDED.is_scam,
                scam_label = EXCLUDED.scam_label,
                scam_mechanism = EXCLUDED.scam_mechanism,
                scam_mechanism_label = EXCLUDED.scam_mechanism_label,
                pool_count = EXCLUDED.pool_count,
                active_pool_count = EXCLUDED.active_pool_count,
                retained_pool_count = EXCLUDED.retained_pool_count,
                liquidity_removal_pool_count = EXCLUDED.liquidity_removal_pool_count,
                updated_at = now()
            "#,
        )
        .bind(&state.scope_id)
        .bind(state.chain_id)
        .bind(&state.token_address)
        .bind(&state.source_run_id)
        .bind(optional_u64_to_i64(state.as_of_block, "as_of_block")?)
        .bind(&state.source_reason)
        .bind(&state.name)
        .bind(&state.symbol)
        .bind(i16::from(state.decimals))
        .bind(&state.total_supply_raw)
        .bind(optional_u64_to_i64(state.creation_block, "creation_block")?)
        .bind(optional_u64_to_i64(
            state.creation_timestamp,
            "creation_timestamp",
        )?)
        .bind(&state.creation_tx)
        .bind(&state.creator_address)
        .bind(optional_u64_to_i64(
            state.latest_activity_block,
            "latest_activity_block",
        )?)
        .bind(optional_u64_to_i64(
            state.latest_activity_timestamp,
            "latest_activity_timestamp",
        )?)
        .bind(optional_u64_to_i64(
            state.latest_pool_activity_block,
            "latest_pool_activity_block",
        )?)
        .bind(&state.lifecycle_status)
        .bind(state.is_scam)
        .bind(&state.scam_label)
        .bind(&state.scam_mechanism)
        .bind(&state.scam_mechanism_label)
        .bind(u64_to_i64(state.pool_count, "pool_count")?)
        .bind(u64_to_i64(state.active_pool_count, "active_pool_count")?)
        .bind(u64_to_i64(
            state.retained_pool_count,
            "retained_pool_count",
        )?)
        .bind(u64_to_i64(
            state.liquidity_removal_pool_count,
            "liquidity_removal_pool_count",
        )?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn write_pool_latest(&self, state: &PoolLatestState) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO token_state.pool_latest (
                scope_id, chain_id, token_address, pool_id, source_run_id, as_of_block,
                source_reason, protocol, denom_address, token_decimals, denom_decimals,
                creation_block, creation_timestamp, creation_tx,
                latest_activity_block, latest_sync_block, latest_reserve_block,
                latest_swap_block, latest_mint_block, latest_burn_block,
                token_reserve, denom_reserve, total_liquidity,
                price_denom_per_token, price_token_per_denom, valuation_status,
                lifecycle_status, is_dust_pool,
                can_buy, can_sell, effective_can_buy, effective_can_sell, economic_sellable,
                buy_tax, sell_tax, tax_check_block,
                has_liquidity_removal, liquidity_removal_block, liquidity_removal_tx,
                liquidity_removal_label, total_swaps, total_mints, total_burns
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14,
                $15, $16, $17,
                $18, $19, $20,
                $21, $22, $23,
                $24, $25, $26,
                $27, $28,
                $29, $30, $31, $32, $33,
                $34, $35, $36,
                $37, $38, $39,
                $40, $41, $42, $43
            )
            ON CONFLICT (scope_id, chain_id, pool_id) DO UPDATE SET
                token_address = EXCLUDED.token_address,
                source_run_id = EXCLUDED.source_run_id,
                as_of_block = EXCLUDED.as_of_block,
                source_reason = EXCLUDED.source_reason,
                protocol = EXCLUDED.protocol,
                denom_address = EXCLUDED.denom_address,
                token_decimals = EXCLUDED.token_decimals,
                denom_decimals = EXCLUDED.denom_decimals,
                creation_block = EXCLUDED.creation_block,
                creation_timestamp = EXCLUDED.creation_timestamp,
                creation_tx = EXCLUDED.creation_tx,
                latest_activity_block = EXCLUDED.latest_activity_block,
                latest_sync_block = EXCLUDED.latest_sync_block,
                latest_reserve_block = EXCLUDED.latest_reserve_block,
                latest_swap_block = EXCLUDED.latest_swap_block,
                latest_mint_block = EXCLUDED.latest_mint_block,
                latest_burn_block = EXCLUDED.latest_burn_block,
                token_reserve = EXCLUDED.token_reserve,
                denom_reserve = EXCLUDED.denom_reserve,
                total_liquidity = EXCLUDED.total_liquidity,
                price_denom_per_token = EXCLUDED.price_denom_per_token,
                price_token_per_denom = EXCLUDED.price_token_per_denom,
                valuation_status = EXCLUDED.valuation_status,
                lifecycle_status = EXCLUDED.lifecycle_status,
                is_dust_pool = EXCLUDED.is_dust_pool,
                can_buy = EXCLUDED.can_buy,
                can_sell = EXCLUDED.can_sell,
                effective_can_buy = EXCLUDED.effective_can_buy,
                effective_can_sell = EXCLUDED.effective_can_sell,
                economic_sellable = EXCLUDED.economic_sellable,
                buy_tax = EXCLUDED.buy_tax,
                sell_tax = EXCLUDED.sell_tax,
                tax_check_block = EXCLUDED.tax_check_block,
                has_liquidity_removal = EXCLUDED.has_liquidity_removal,
                liquidity_removal_block = EXCLUDED.liquidity_removal_block,
                liquidity_removal_tx = EXCLUDED.liquidity_removal_tx,
                liquidity_removal_label = EXCLUDED.liquidity_removal_label,
                total_swaps = EXCLUDED.total_swaps,
                total_mints = EXCLUDED.total_mints,
                total_burns = EXCLUDED.total_burns,
                updated_at = now()
            "#,
        )
        .bind(&state.scope_id)
        .bind(state.chain_id)
        .bind(&state.token_address)
        .bind(&state.pool_id)
        .bind(&state.source_run_id)
        .bind(optional_u64_to_i64(state.as_of_block, "as_of_block")?)
        .bind(&state.source_reason)
        .bind(&state.protocol)
        .bind(&state.denom_address)
        .bind(i16::from(state.token_decimals))
        .bind(i16::from(state.denom_decimals))
        .bind(optional_u64_to_i64(state.creation_block, "creation_block")?)
        .bind(optional_u64_to_i64(
            state.creation_timestamp,
            "creation_timestamp",
        )?)
        .bind(&state.creation_tx)
        .bind(optional_u64_to_i64(
            state.latest_activity_block,
            "latest_activity_block",
        )?)
        .bind(optional_u64_to_i64(
            state.latest_sync_block,
            "latest_sync_block",
        )?)
        .bind(optional_u64_to_i64(
            state.latest_reserve_block,
            "latest_reserve_block",
        )?)
        .bind(optional_u64_to_i64(
            state.latest_swap_block,
            "latest_swap_block",
        )?)
        .bind(optional_u64_to_i64(
            state.latest_mint_block,
            "latest_mint_block",
        )?)
        .bind(optional_u64_to_i64(
            state.latest_burn_block,
            "latest_burn_block",
        )?)
        .bind(state.token_reserve)
        .bind(state.denom_reserve)
        .bind(state.total_liquidity)
        .bind(state.price_denom_per_token)
        .bind(state.price_token_per_denom)
        .bind(&state.valuation_status)
        .bind(&state.lifecycle_status)
        .bind(state.is_dust_pool)
        .bind(state.can_buy)
        .bind(state.can_sell)
        .bind(state.effective_can_buy)
        .bind(state.effective_can_sell)
        .bind(state.economic_sellable)
        .bind(state.buy_tax)
        .bind(state.sell_tax)
        .bind(optional_u64_to_i64(
            state.tax_check_block,
            "tax_check_block",
        )?)
        .bind(state.has_liquidity_removal)
        .bind(optional_u64_to_i64(
            state.liquidity_removal_block,
            "liquidity_removal_block",
        )?)
        .bind(&state.liquidity_removal_tx)
        .bind(&state.liquidity_removal_label)
        .bind(u64_to_i64(state.total_swaps, "total_swaps")?)
        .bind(u64_to_i64(state.total_mints, "total_mints")?)
        .bind(u64_to_i64(state.total_burns, "total_burns")?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
