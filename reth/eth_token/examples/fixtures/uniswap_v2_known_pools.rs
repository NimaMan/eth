use alloy_primitives::{address, Address};

#[derive(Clone, Copy, Debug)]
pub struct KnownUniswapV2PoolRange {
    pub name: &'static str,
    pub token: Address,
    pub pool: Address,
    pub start_block: u64,
    pub end_block: u64,
    pub note: &'static str,
}

pub const MOO_WETH_LAUNCH: KnownUniswapV2PoolRange = KnownUniswapV2PoolRange {
    name: "moo_weth_launch",
    token: address!("DF6010eF80142D379eA0324ac100Dd3Cf50901b2"),
    pool: address!("Fc099D07b32D52D61d2f5Dd6De2614d26474eCf7"),
    start_block: 23_196_199,
    end_block: 23_196_202,
    note: "MOO/WETH launch range with liquidity creation and early reserve syncs.",
};

pub const KNOWN_UNISWAP_V2_POOL_RANGES: &[KnownUniswapV2PoolRange] = &[MOO_WETH_LAUNCH];

pub fn find_known_uniswap_v2_pool_range(name: &str) -> Option<KnownUniswapV2PoolRange> {
    KNOWN_UNISWAP_V2_POOL_RANGES
        .iter()
        .copied()
        .find(|range| range.name == name)
}
