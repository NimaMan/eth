use alloy_primitives::{Address, U256};
use serde_json::{json, Value};

pub(super) fn normalize_evidence_protocol(protocol: &str) -> String {
    match protocol.trim().to_ascii_uppercase().as_str() {
        "UNISWAPV2" | "UNISWAP-V2" | "V2" => "UNISWAP-V2",
        "SUSHISWAP" | "SUSHISWAP-V2" | "SUSHI" | "SUSHI-V2" => "SUSHISWAP-V2",
        "PANCAKESWAP" | "PANCAKESWAP-V2" | "PANCAKE-V2" => "PANCAKESWAP-V2",
        "SHIBASWAP" | "SHIBASWAP-V2" => "SHIBASWAP-V2",
        "FRAXSWAP" | "FRAXSWAP-V2" => "FRAXSWAP-V2",
        other => other,
    }
    .to_string()
}

pub(super) fn pool_type_label(pool_type: tx_processor::PoolType) -> &'static str {
    match pool_type {
        tx_processor::PoolType::UniswapV2 => "UNISWAP-V2",
        tx_processor::PoolType::SushiSwap => "SUSHISWAP-V2",
        tx_processor::PoolType::PancakeSwapV2 => "PANCAKESWAP-V2",
        tx_processor::PoolType::ShibaSwapV2 => "SHIBASWAP-V2",
        tx_processor::PoolType::FraxswapV2 => "FRAXSWAP-V2",
        tx_processor::PoolType::UniswapV3 { .. } => "UNISWAP-V3",
        tx_processor::PoolType::SushiSwapV3 { .. } => "SUSHISWAP-V3",
        tx_processor::PoolType::PancakeSwapV3 { .. } => "PANCAKESWAP-V3",
        tx_processor::PoolType::Curve => "CURVE",
        tx_processor::PoolType::Balancer => "BALANCER",
        tx_processor::PoolType::UniswapV4 => "UNISWAP-V4",
    }
}

pub(super) fn denom_symbol_for_address(address: &str) -> String {
    if address.eq_ignore_ascii_case("0x0000000000000000000000000000000000000000") {
        "ETH".to_string()
    } else if address.eq_ignore_ascii_case("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2") {
        "WETH".to_string()
    } else {
        address.to_string()
    }
}

pub(super) fn tax_value(value: f64) -> Value {
    if value.is_finite() && value >= 0.0 {
        json!(format_f64_decimal(value))
    } else {
        Value::Null
    }
}

pub(super) fn parse_address(value: &str) -> Option<Address> {
    value.trim().parse::<Address>().ok()
}

pub(super) fn format_f64_decimal(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let mut text = format!("{value:.18}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text.is_empty() {
        "0".to_string()
    } else {
        text
    }
}

pub(super) fn u256_to_decimal_string(value: U256, decimals: u8) -> String {
    let decimals = usize::from(decimals);
    let raw = value.to_string();
    if decimals == 0 {
        return raw;
    }
    let mut text = if raw.len() <= decimals {
        let mut padded = String::with_capacity(decimals + 2);
        padded.push_str("0.");
        for _ in 0..(decimals - raw.len()) {
            padded.push('0');
        }
        padded.push_str(&raw);
        padded
    } else {
        let split = raw.len() - decimals;
        format!("{}.{}", &raw[..split], &raw[split..])
    };
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

pub(super) fn known_denom_decimals(symbol: &str, address: &str) -> Option<u8> {
    match symbol.trim().to_ascii_uppercase().as_str() {
        "ETH" | "WETH" | "DAI" | "USDE" => Some(18),
        "USDC" | "USDT" | "EUROC" | "EURC" => Some(6),
        _ if address.eq_ignore_ascii_case("0x0000000000000000000000000000000000000000") => Some(18),
        _ if address.eq_ignore_ascii_case("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2") => Some(18),
        _ if address.eq_ignore_ascii_case("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48") => Some(6),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::U256;
    use rust_decimal::Decimal;

    use super::u256_to_decimal_string;

    #[test]
    fn token_reserve_projection_scales_raw_v2_reserves() {
        let raw = U256::from_str_radix("881000000000000000000000000000000", 10).unwrap();
        assert!(Decimal::from_str_exact(&raw.to_string()).is_err());

        let scaled = u256_to_decimal_string(raw, 18);
        assert_eq!(scaled, "881000000000000");
        assert!(Decimal::from_str_exact(&scaled).is_ok());
    }
}
