//! Central address book shared across the crate.
//!
//! `ADDRESSES_BY_NAME` collects all named addresses (routers, wallets, tokens,
//! CEX/ETF labels, etc.) so downstream code can resolve a human-readable label
//! to an `Address` without duplicating per-category maps.

use crate::common_addresses::{
    burn_addresses::{DEAD_ADDRESS, ZERO_ADDRESS},
    cex::CEX_ADDRESSES,
    denom_tokens::SYMBOL_TO_ADDRESS,
    etf::ETF_ADDRESSES,
    wallets::WALLET_ADDRESSES,
};
use alloy_primitives::{address, Address};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Static list of well-known routers/contracts that are not part of another dataset.
const BASE_ADDRESSES: &[(&str, Address)] = &[
    ("zero_address", ZERO_ADDRESS),
    ("dead_address", DEAD_ADDRESS),
    ("WETH", address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")),
    ("ETH", address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")),
    ("WBTC", address!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599")),
    (
        "TrustSwap: Team Finance Lock",
        address!("E2fE530C047f2d85298b07D9333C05737f1435fB"),
    ),
    (
        "UNCX Network Security: LP Lockers",
        address!("663A5C229c09b049E36dCc11a9B0d4a8Eb9db214"),
    ),
    (
        "UNCX Network Security: Token Vesting",
        address!("Dba68f07d1b7Ca219f78ae8582C213d975c25cAf"),
    ),
    (
        "UNCX Network Lockers: V3 Proof of Reserves 2",
        address!("7f5C649856F900d15C83741f45AE46f5C6858234"),
    ),
    (
        "UNCX Network Lockers: V3 Proof of Reserves 3",
        address!("FD235968e65B0990584585763f837A5b5330e6DE"),
    ),
    (
        "PinkLock02",
        address!("71B5759d73262FBb223956913ecF4ecC51057641"),
    ),
    (
        "UniswapV2Router02",
        address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
    ),
    (
        "UniswapV2Factory",
        address!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
    ),
    (
        "UniswapV3Factory",
        address!("1F98431c8aD98523631AE4a59f267346ea31F984"),
    ),
    (
        "UniswapUniversalRouter",
        address!("3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD"),
    ),
    (
        "NonfungiblePositionManager",
        address!("C36442b4a4522E871399CD717aBDD847Ab11FE88"),
    ),
    (
        "WETH_USDT",
        address!("11b815efB8f581194ae79006d24E0d814B7697F6"),
    ),
    (
        "Banana Gun: Router 2",
        address!("3328F7f4A1D1C57c35df56bBf0c9dCAFCA309C49"),
    ),
    (
        "Banana Gun: Deployer",
        address!("BCd3a47e4d0000cf170E25d1bD3d53F7C08be0A6"),
    ),
    (
        "Banana Gun: Deployer 2",
        address!("35fC556d6f8675B26fDF1542e6E894100155B34E"),
    ),
    (
        "Banana Gun",
        address!("C465CC50B7D5A29b9308968f870a4B242A8e1873"),
    ),
    (
        "Maestro: Router 2",
        address!("80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e"),
    ),
    (
        "Maestro: Deployer",
        address!("6599aE06914f1f5Ec0053d3F475348D40E608442"),
    ),
    (
        "Unibot",
        address!("5c9321e92Ba4eb43f2901c4952358e132163a85A"),
    ),
    (
        "Sigma / Alphaman",
        address!("e76014c179F19dA26Bb30A0f085FF0A466B92829"),
    ),
    (
        "Metamask: Swap Router",
        address!("881D40237659C251811CEC9c364ef91dC08D300C"),
    ),
    (
        "1inch v5: Aggregation Router",
        address!("1111111254EEB25477B68fb85Ed929f73A960582"),
    ),
];

/// Master map of names to addresses used by the rest of the crate.
pub static ADDRESSES_BY_NAME: Lazy<HashMap<&'static str, Address>> = Lazy::new(|| {
    let mut map: HashMap<&'static str, Address> = HashMap::new();

    for (label, address) in BASE_ADDRESSES {
        map.insert(*label, *address);
    }

    for (label, address) in WALLET_ADDRESSES.iter() {
        map.insert(*label, *address);
    }

    // Include token symbols (stablecoins, denoms, etc.)
    for (symbol, address) in SYMBOL_TO_ADDRESS.iter() {
        map.entry(*symbol).or_insert(*address);
    }

    // Include every CEX label
    for entry in CEX_ADDRESSES.iter() {
        map.entry(entry.name).or_insert(entry.address);
    }

    // Include every ETF label
    for entry in ETF_ADDRESSES.iter() {
        map.entry(entry.name).or_insert(entry.address);
    }

    map
});

/// Resolve an address by human-readable label.
pub fn get_address_by_name(name: &str) -> Option<Address> {
    ADDRESSES_BY_NAME.get(name).copied()
}
