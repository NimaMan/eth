use sqlx::Row;

#[derive(Clone, Debug)]
pub struct RiskAtlasObservation {
    pub token_address: String,
    pub pool_address: String,
    pub denom_address: String,
    pub protocol: String,
    pub block_number: i64,
    pub denom_reserve: Option<f64>,
    pub token_reserve: Option<f64>,
    pub can_buy: bool,
    pub effective_can_buy: Option<bool>,
    pub can_sell: bool,
    pub effective_can_sell: Option<bool>,
    pub direct_lp_removal_in_block: bool,
    pub quote_symbol: Option<String>,
    pub token_decimals: Option<i32>,
    pub price_denom_per_token: Option<f64>,
    pub initial_price_denom_per_token: Option<f64>,
    pub price_ratio_to_initial: Option<f64>,
    pub creation_block: Option<i64>,
    pub lp_approval_count_in_block: i32,
    pub lp_total_supply: Option<f64>,
    pub lp_max_approval_amount_as_of: Option<f64>,
    pub lp_approval_owner_is_creator: Option<bool>,
}

pub async fn query_risk_atlas_observations(
    pool: &sqlx::PgPool,
    risk_atlas_run_id: &str,
    from_block: Option<u64>,
    to_block: Option<u64>,
    allowed_protocols: &[String],
) -> sqlx::Result<Vec<RiskAtlasObservation>> {
    let mut query = String::from(
        r#"
        SELECT
            o.token_address,
            o.pool_address,
            o.denom_address,
            o.protocol,
            o.block_number,
            o.denom_reserve,
            o.token_reserve,
            o.can_buy,
            o.effective_can_buy,
            o.can_sell,
            o.effective_can_sell,
            o.direct_lp_removal_in_block,
            pe.quote_symbol,
            COALESCE(o.token_decimals, NULLIF(o.features->'token'->>'decimals', '')::INT) AS token_decimals,
            COALESCE(o.price_denom_per_token, NULLIF(o.features->'liquidity'->>'price_denom_per_token', '')::DOUBLE PRECISION) AS price_denom_per_token,
            COALESCE(o.initial_price_denom_per_token, NULLIF(o.features->'liquidity'->>'initial_price_denom_per_token', '')::DOUBLE PRECISION) AS initial_price_denom_per_token,
            COALESCE(o.price_to_initial_ratio, NULLIF(o.features->'liquidity'->>'price_to_initial_ratio', '')::DOUBLE PRECISION) AS price_ratio_to_initial,
            pe.first_observed_block AS creation_block,
            GREATEST(
                COALESCE(o.lp_approval_count_in_block, 0),
                COALESCE(NULLIF(o.observation->'event_flags'->>'lp_approval_count_in_block', '')::INT, 0)
            ) AS lp_approval_count_in_block,
            COALESCE(o.lp_total_supply, NULLIF(o.features->'lp_control'->>'lp_total_supply', '')::DOUBLE PRECISION) AS lp_total_supply,
            COALESCE(o.lp_max_approval_amount_as_of, NULLIF(o.features->'lp_control'->>'lp_max_approval_amount_as_of', '')::DOUBLE PRECISION) AS lp_max_approval_amount_as_of,
            COALESCE(o.lp_approval_owner_is_creator, NULLIF(o.features->'lp_control'->>'last_lp_approval_owner_is_creator', '')::BOOLEAN) AS lp_approval_owner_is_creator
        FROM risk_atlas_observations o
        JOIN risk_atlas_pool_eligibility pe
          ON pe.run_id = o.run_id
         AND pe.token_address = o.token_address
         AND pe.pool_address = o.pool_address
        WHERE o.run_id = $1
          AND pe.eligible
        "#,
    );
    let mut next_bind = 2;
    if !allowed_protocols.is_empty() {
        query.push_str(&format!(" AND o.protocol = ANY(${next_bind})"));
        next_bind += 1;
    }
    if from_block.is_some() {
        query.push_str(&format!(" AND o.block_number >= ${next_bind}"));
        next_bind += 1;
    }
    if to_block.is_some() {
        query.push_str(&format!(" AND o.block_number <= ${next_bind}"));
    }
    query.push_str(" ORDER BY o.block_number ASC, o.token_address ASC, o.pool_address ASC");

    let mut q = sqlx::query(&query).bind(risk_atlas_run_id);
    if !allowed_protocols.is_empty() {
        q = q.bind(allowed_protocols);
    }
    if let Some(block) = from_block {
        q = q.bind(block as i64);
    }
    if let Some(block) = to_block {
        q = q.bind(block as i64);
    }

    let rows = q.fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(RiskAtlasObservation {
                token_address: row.try_get("token_address")?,
                pool_address: row.try_get("pool_address")?,
                denom_address: row.try_get("denom_address")?,
                protocol: row.try_get("protocol")?,
                block_number: row.try_get("block_number")?,
                denom_reserve: row.try_get("denom_reserve")?,
                token_reserve: row.try_get("token_reserve")?,
                can_buy: row.try_get("can_buy")?,
                effective_can_buy: row.try_get("effective_can_buy")?,
                can_sell: row.try_get("can_sell")?,
                effective_can_sell: row.try_get("effective_can_sell")?,
                direct_lp_removal_in_block: row.try_get("direct_lp_removal_in_block")?,
                quote_symbol: row.try_get("quote_symbol")?,
                token_decimals: row.try_get("token_decimals")?,
                price_denom_per_token: row.try_get("price_denom_per_token")?,
                initial_price_denom_per_token: row.try_get("initial_price_denom_per_token")?,
                price_ratio_to_initial: row.try_get("price_ratio_to_initial")?,
                creation_block: row.try_get("creation_block")?,
                lp_approval_count_in_block: row.try_get("lp_approval_count_in_block")?,
                lp_total_supply: row.try_get("lp_total_supply")?,
                lp_max_approval_amount_as_of: row.try_get("lp_max_approval_amount_as_of")?,
                lp_approval_owner_is_creator: row.try_get("lp_approval_owner_is_creator")?,
            })
        })
        .collect()
}
