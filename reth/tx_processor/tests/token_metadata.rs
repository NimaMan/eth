use alloy_primitives::address;
use reth_chain_query::{
    common_addresses::denom_tokens::{get_token_decimals, get_token_symbol},
    RethQueryProvider,
};

#[test]
fn denom_tokens_have_expected_metadata() {
    let cases = vec![
        (address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"), "USDC", 6),
        (address!("6B175474E89094C44Da98b954EedeAC495271d0F"), "DAI", 18),
        (address!("C02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"), "WETH", 18),
        (address!("dAC17F958D2ee523a2206206994597C13D831ec7"), "USDT", 6),
        (address!("853d955aCEf822Db058eb8505911ED77F175b99e"), "FRAX", 18),
    ];

    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")
        .expect("failed to open Reth datadir for metadata tests");

    for (addr, expected_symbol, expected_decimals) in cases {
        let symbol = get_token_symbol(addr)
            .unwrap_or_else(|| panic!("symbol missing for address {addr:?}"));
        assert_eq!(
            symbol, expected_symbol,
            "unexpected symbol for address {addr:?}"
        );

        let decimals = get_token_decimals(symbol)
            .unwrap_or_else(|| panic!("decimals missing for symbol {symbol}"));
        assert_eq!(
            decimals, expected_decimals,
            "unexpected decimals for symbol {symbol}"
        );

        let metadata =                                 
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(provider.get_token_metadata(addr, Some(23848765), &[]))
                .unwrap_or_else(|err| panic!("metadata fetch failed for {addr:?}: {err}"))
                .expect("metadata is None");
        assert_eq!(metadata.symbol, expected_symbol);
        assert_eq!(metadata.decimals, expected_decimals);
    }
}
