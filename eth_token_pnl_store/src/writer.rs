use eth_token::pnl::{PnlAddressPositionExport, PnlMovementExport, PnlPoolExport};
use sqlx::{PgPool, Postgres, Transaction};

use crate::{
    error::u64_to_i64, migration::run_migrations, schema::PnlCalculationRun, Result,
    TokenPnlStoreConfig,
};

#[derive(Clone)]
pub struct TokenPnlStore {
    pool: PgPool,
}

impl TokenPnlStore {
    pub async fn connect(config: &TokenPnlStoreConfig) -> Result<Self> {
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

    pub async fn upsert_run(&self, run: &PnlCalculationRun) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO token_pnl.calculation_runs (
                run_id, chain_id, mode, algorithm_version, start_block, end_block,
                include_traces, status, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (run_id) DO UPDATE SET
                chain_id = EXCLUDED.chain_id,
                mode = EXCLUDED.mode,
                algorithm_version = EXCLUDED.algorithm_version,
                start_block = EXCLUDED.start_block,
                end_block = EXCLUDED.end_block,
                include_traces = EXCLUDED.include_traces,
                status = EXCLUDED.status,
                metadata = EXCLUDED.metadata,
                updated_at = now()
            "#,
        )
        .bind(&run.run_id)
        .bind(run.chain_id)
        .bind(&run.mode)
        .bind(&run.algorithm_version)
        .bind(run.start_block)
        .bind(run.end_block)
        .bind(run.include_traces)
        .bind(&run.status)
        .bind(&run.metadata)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn write_pool_export(&self, run_id: &str, export: &PnlPoolExport) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        upsert_pool_state(&mut tx, run_id, export).await?;
        replace_address_positions(&mut tx, run_id, export).await?;
        replace_movements(&mut tx, run_id, export).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn write_pool_aggregate_export(
        &self,
        run_id: &str,
        export: &PnlPoolExport,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        upsert_pool_state(&mut tx, run_id, export).await?;
        replace_address_positions(&mut tx, run_id, export).await?;
        clear_movements(&mut tx, run_id, export).await?;
        tx.commit().await?;
        Ok(())
    }
}

async fn upsert_pool_state(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    export: &PnlPoolExport,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO token_pnl.pool_pnl_states (
            run_id, pool_id, token_address, denom_address, protocol,
            token_decimals, denom_decimals, tx_count, latest_block, latest_timestamp,
            token_in_raw, token_out_raw, denom_in_raw, denom_out_raw,
            pool_token_in_raw, pool_token_out_raw, pool_denom_in_raw, pool_denom_out_raw,
            native_fee_raw, native_bribe_raw, token_transfer_count, denom_transfer_count,
            token_creator_address, pool_creator_address,
            can_buy, can_sell, lifecycle, is_scam, scam_label, eligible, eligible_outcome
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10,
            $11::numeric(78,0), $12::numeric(78,0), $13::numeric(78,0), $14::numeric(78,0),
            $15::numeric(78,0), $16::numeric(78,0), $17::numeric(78,0), $18::numeric(78,0),
            $19::numeric(78,0), $20::numeric(78,0), $21, $22,
            $23, $24,
            $25, $26, $27, $28, $29, $30, $31
        )
        ON CONFLICT (run_id, pool_id) DO UPDATE SET
            token_address = EXCLUDED.token_address,
            denom_address = EXCLUDED.denom_address,
            protocol = EXCLUDED.protocol,
            token_decimals = EXCLUDED.token_decimals,
            denom_decimals = EXCLUDED.denom_decimals,
            tx_count = EXCLUDED.tx_count,
            latest_block = EXCLUDED.latest_block,
            latest_timestamp = EXCLUDED.latest_timestamp,
            token_in_raw = EXCLUDED.token_in_raw,
            token_out_raw = EXCLUDED.token_out_raw,
            denom_in_raw = EXCLUDED.denom_in_raw,
            denom_out_raw = EXCLUDED.denom_out_raw,
            pool_token_in_raw = EXCLUDED.pool_token_in_raw,
            pool_token_out_raw = EXCLUDED.pool_token_out_raw,
            pool_denom_in_raw = EXCLUDED.pool_denom_in_raw,
            pool_denom_out_raw = EXCLUDED.pool_denom_out_raw,
            native_fee_raw = EXCLUDED.native_fee_raw,
            native_bribe_raw = EXCLUDED.native_bribe_raw,
            token_transfer_count = EXCLUDED.token_transfer_count,
            denom_transfer_count = EXCLUDED.denom_transfer_count,
            token_creator_address = EXCLUDED.token_creator_address,
            pool_creator_address = EXCLUDED.pool_creator_address,
            can_buy = EXCLUDED.can_buy,
            can_sell = EXCLUDED.can_sell,
            lifecycle = EXCLUDED.lifecycle,
            is_scam = EXCLUDED.is_scam,
            scam_label = EXCLUDED.scam_label,
            eligible = EXCLUDED.eligible,
            eligible_outcome = EXCLUDED.eligible_outcome,
            updated_at = now()
        "#,
    )
    .bind(run_id)
    .bind(&export.pool_id)
    .bind(&export.token_address)
    .bind(&export.denom_address)
    .bind(&export.protocol)
    .bind(i16::from(export.token_decimals))
    .bind(i16::from(export.denom_decimals))
    .bind(u64_to_i64(export.tx_count, "tx_count")?)
    .bind(optional_u64_to_i64(
        export.latest_block_number,
        "latest_block",
    )?)
    .bind(optional_u64_to_i64(
        export.latest_block_timestamp,
        "latest_timestamp",
    )?)
    .bind(&export.conservation.token_in_raw)
    .bind(&export.conservation.token_out_raw)
    .bind(&export.conservation.denom_in_raw)
    .bind(&export.conservation.denom_out_raw)
    .bind(&export.conservation.pool_token_in_raw)
    .bind(&export.conservation.pool_token_out_raw)
    .bind(&export.conservation.pool_denom_in_raw)
    .bind(&export.conservation.pool_denom_out_raw)
    .bind(&export.conservation.native_fee_raw)
    .bind(&export.conservation.native_bribe_raw)
    .bind(u64_to_i64(
        export.conservation.token_transfer_count,
        "token_transfer_count",
    )?)
    .bind(u64_to_i64(
        export.conservation.denom_transfer_count,
        "denom_transfer_count",
    )?)
    .bind(&export.meta.token_creator_address)
    .bind(&export.meta.pool_creator_address)
    .bind(export.meta.can_buy)
    .bind(export.meta.can_sell)
    .bind(&export.meta.lifecycle)
    .bind(export.meta.is_scam)
    .bind(&export.meta.scam_label)
    .bind(export.meta.eligible)
    .bind(&export.meta.eligible_outcome)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn replace_address_positions(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    export: &PnlPoolExport,
) -> Result<()> {
    sqlx::query("DELETE FROM token_pnl.pool_address_pnl WHERE run_id = $1 AND pool_id = $2")
        .bind(run_id)
        .bind(&export.pool_id)
        .execute(&mut **tx)
        .await?;

    for position in &export.address_positions {
        insert_address_position(tx, run_id, &export.pool_id, position).await?;
    }
    Ok(())
}

async fn insert_address_position(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    pool_id: &str,
    position: &PnlAddressPositionExport,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO token_pnl.pool_address_pnl (
            run_id, pool_id, address,
            token_in_raw, token_out_raw, denom_in_raw, denom_out_raw,
            native_fee_raw, native_bribe_raw, token_balance_raw, denom_cashflow_raw,
            token_balance, denom_cashflow, native_fee, native_bribe,
            marked_token_value_denom, pnl_proxy_denom,
            first_block, latest_block, movement_count
        )
        VALUES (
            $1, $2, $3,
            $4::numeric(78,0), $5::numeric(78,0), $6::numeric(78,0), $7::numeric(78,0),
            $8::numeric(78,0), $9::numeric(78,0), $10, $11,
            $12, $13, $14, $15,
            $16, $17,
            $18, $19, $20
        )
        "#,
    )
    .bind(run_id)
    .bind(pool_id)
    .bind(&position.address)
    .bind(&position.token_in_raw)
    .bind(&position.token_out_raw)
    .bind(&position.denom_in_raw)
    .bind(&position.denom_out_raw)
    .bind(&position.native_fee_raw)
    .bind(&position.native_bribe_raw)
    .bind(&position.token_balance_raw)
    .bind(&position.denom_cashflow_raw)
    .bind(position.token_balance)
    .bind(position.denom_cashflow)
    .bind(position.native_fee)
    .bind(position.native_bribe)
    .bind(position.marked_token_value_denom)
    .bind(position.pnl_proxy_denom)
    .bind(optional_u64_to_i64(position.first_block, "first_block")?)
    .bind(optional_u64_to_i64(position.latest_block, "latest_block")?)
    .bind(u64_to_i64(position.movement_count, "movement_count")?)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn replace_movements(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    export: &PnlPoolExport,
) -> Result<()> {
    clear_movements(tx, run_id, export).await?;

    for movement in &export.movements {
        insert_movement(tx, run_id, &export.pool_id, movement).await?;
    }
    Ok(())
}

async fn clear_movements(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    export: &PnlPoolExport,
) -> Result<()> {
    sqlx::query("DELETE FROM token_pnl.pool_pnl_movements WHERE run_id = $1 AND pool_id = $2")
        .bind(run_id)
        .bind(&export.pool_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn insert_movement(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    pool_id: &str,
    movement: &PnlMovementExport,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO token_pnl.pool_pnl_movements (
            run_id, pool_id, entry_index, tx_hash, block_number, block_timestamp,
            tx_index, log_index, address, kind,
            token_in_raw, token_out_raw, denom_in_raw, denom_out_raw,
            native_fee_raw, native_bribe_raw, pool_direct
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10,
            $11::numeric(78,0), $12::numeric(78,0), $13::numeric(78,0), $14::numeric(78,0),
            $15::numeric(78,0), $16::numeric(78,0), $17
        )
        "#,
    )
    .bind(run_id)
    .bind(pool_id)
    .bind(u64_to_i64(movement.entry_index, "entry_index")?)
    .bind(&movement.tx_hash)
    .bind(u64_to_i64(movement.block_number, "block_number")?)
    .bind(u64_to_i64(movement.block_timestamp, "block_timestamp")?)
    .bind(u64_to_i64(movement.tx_index, "tx_index")?)
    .bind(optional_u64_to_i64(movement.log_index, "log_index")?)
    .bind(&movement.address)
    .bind(&movement.kind)
    .bind(&movement.token_in_raw)
    .bind(&movement.token_out_raw)
    .bind(&movement.denom_in_raw)
    .bind(&movement.denom_out_raw)
    .bind(&movement.native_fee_raw)
    .bind(&movement.native_bribe_raw)
    .bind(movement.pool_direct)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn optional_u64_to_i64(value: Option<u64>, field: &'static str) -> Result<Option<i64>> {
    value.map(|value| u64_to_i64(value, field)).transpose()
}
