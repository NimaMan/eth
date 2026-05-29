use alloy_primitives::U256;

use crate::token_tracking::{Pool, PoolType};

use super::{pool_type_label, LpApprovalSignal};

const LIQUIDITY_OWNERSHIP_TOKEN_DECIMALS: i32 = 18;

pub(crate) fn enrich_erc20_liquidity_approval(
    mut signal: LpApprovalSignal,
    pool: &Pool,
) -> LpApprovalSignal {
    signal.token_address = pool.token_address.clone();
    signal.pool_address = pool.address.clone();
    signal.lp_token_address = pool
        .lp_token_address
        .clone()
        .unwrap_or_else(|| signal.lp_token_address.clone());
    signal.pool_type = pool_type_label(&pool.pool_type);
    signal.denom_address = Some(pool.denom_address.clone());
    signal.denom_currency = Some(pool.denom_currency.clone());
    signal.denom_decimals = None;
    signal.lp_total_supply = pool.lp_total_supply.map(format_lp_supply);
    signal.approval_model = Some(approval_model_for_pool(&pool.pool_type).to_string());
    if pool.last_updated_block > 0 {
        signal.detected_at_head_block_number = Some(pool.last_updated_block);
    }

    if let Some(share_pct) = approved_share_pct(signal.amount, pool.lp_total_supply) {
        signal.approval_percentage = Some(share_pct);
        signal.approved_share_pct = Some(share_pct);
    }

    signal
}

fn approved_share_pct(amount: U256, lp_total_supply: Option<f64>) -> Option<f64> {
    let total_supply = lp_total_supply?;
    if !total_supply.is_finite() || total_supply <= 0.0 {
        return None;
    }

    let amount =
        amount.to_string().parse::<f64>().ok()? / 10f64.powi(LIQUIDITY_OWNERSHIP_TOKEN_DECIMALS);
    if !amount.is_finite() {
        return None;
    }

    Some(normalize_percent((amount / total_supply) * 100.0))
}

fn normalize_percent(value: f64) -> f64 {
    if !value.is_finite() {
        return 100.0;
    }
    let value = value.clamp(0.0, 100.0);
    if value.abs() < 0.0000001 {
        0.0
    } else {
        value
    }
}

fn format_lp_supply(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        let formatted = format!("{value:.18}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn approval_model_for_pool(pool_type: &PoolType) -> &'static str {
    match pool_type {
        PoolType::Balancer => "balancer_bpt_share",
        PoolType::Curve => "curve_lp_share",
        PoolType::UniswapV3
        | PoolType::SushiSwapV3
        | PoolType::PancakeSwapV3
        | PoolType::UniswapV4 => "position_liquidity_share",
        _ => "erc20_lp_share",
    }
}

#[cfg(test)]
mod tests {
    use super::approved_share_pct;
    use alloy_primitives::U256;

    #[test]
    fn computes_approval_share_from_raw_18_decimal_amount() {
        let one_lp_token = U256::from(10u128.pow(18));

        assert_eq!(approved_share_pct(one_lp_token, Some(4.0)), Some(25.0));
    }

    #[test]
    fn caps_unbounded_approval_at_full_pool_share() {
        assert_eq!(approved_share_pct(U256::MAX, Some(4.0)), Some(100.0));
    }
}
