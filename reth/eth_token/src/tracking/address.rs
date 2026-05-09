use alloy_primitives::{Address, B256};

pub(crate) fn same_address_str(address: Address, value: &str) -> bool {
    address_string(&address) == normalize_address(value)
}

pub(crate) fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

pub(crate) fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

pub(crate) fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

pub(crate) fn normalize_address_string(value: impl Into<String>) -> String {
    normalize_address(value.into())
}

pub(crate) fn parse_address_lossy(value: &str) -> Address {
    value.parse().unwrap_or(Address::ZERO)
}
