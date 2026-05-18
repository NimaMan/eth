use alloy_primitives::{address, Address};

#[derive(Clone, Copy, Debug)]
pub struct KnownPnlFixture {
    pub name: &'static str,
    pub token: Address,
    pub pool: Address,
    pub denom: Address,
    pub start_block: u64,
    pub end_block: u64,
    pub token_symbol: &'static str,
    pub denom_symbol: &'static str,
    pub protocol: &'static str,
    pub source_url: &'static str,
    pub note: &'static str,
}

pub const HODL_WETH_MAY_2026: KnownPnlFixture = KnownPnlFixture {
    name: "hodl_weth_may_2026",
    token: address!("538F76361ad5e94f21dB670e07f3b4DfF186AF3F"),
    pool: address!("b1440adcAc60dCd82d7F40205DaF5f2cC96Edc61"),
    denom: address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
    start_block: 25_106_317,
    end_block: 25_106_381,
    token_symbol: "HODL",
    denom_symbol: "WETH",
    protocol: "uniswap_v2",
    source_url: "https://dexscreener.com/ethereum/0x538F76361ad5e94f21dB670e07f3b4DfF186AF3F",
    note: "HODL/WETH V2 launch/drain range seeded from DexScreener and Etherscan pair view.",
};

pub const KNOWN_PNL_FIXTURES: &[KnownPnlFixture] = &[HODL_WETH_MAY_2026];

pub fn find_known_pnl_fixture(name: &str) -> Option<KnownPnlFixture> {
    KNOWN_PNL_FIXTURES
        .iter()
        .copied()
        .find(|fixture| fixture.name == name)
}
