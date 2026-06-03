use alloy_primitives::{Address, B256, U256};
use rust_decimal::{Decimal, MathematicalOps};

pub(crate) const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
pub(crate) const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

pub(crate) fn signed_raw_string(incoming: U256, outgoing: U256) -> String {
    if incoming >= outgoing {
        (incoming - outgoing).to_string()
    } else {
        format!("-{}", outgoing - incoming)
    }
}

pub(crate) fn scaled_signed_balance(incoming: U256, outgoing: U256, decimals: u8) -> Decimal {
    if incoming >= outgoing {
        scaled_units(incoming - outgoing, decimals)
    } else {
        -scaled_units(outgoing - incoming, decimals)
    }
}

pub(crate) fn scaled_units(value: U256, decimals: u8) -> Decimal {
    let raw = value
        .to_string()
        .parse::<Decimal>()
        .unwrap_or(Decimal::ZERO);
    let divisor = Decimal::TEN
        .checked_powu(u64::from(decimals))
        .unwrap_or(Decimal::MAX);
    raw.checked_div(divisor).unwrap_or(Decimal::ZERO)
}

pub(crate) fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

pub(crate) fn normalize_address_string(value: String) -> String {
    value.trim().to_ascii_lowercase()
}

pub(crate) fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

pub(crate) fn parse_address(value: &str) -> Option<Address> {
    value.parse::<Address>().ok()
}

pub(crate) fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}
