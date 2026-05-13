//! Liquidity thresholds aligned with `eth_token/erc20_token/config/scam_thresholds.py`.
//!
//! We replicate the denomination thresholds here so the mempool cache applies the same
//! liquidity gating. Thresholds are expressed in native units
//! for the denomination token (ETH, stablecoin units, etc.).

const ETH_CATEGORY: f64 = 0.05; // 0.05 ETH (~$200 at $4k/ETH)
const USD_CATEGORY: f64 = 200.0; // $200 worth of liquidity
const EUR_CATEGORY: f64 = 200.0;
const JPY_CATEGORY: f64 = 30_000.0;
const SGD_CATEGORY: f64 = 270.0;
const CAD_CATEGORY: f64 = 270.0;
const BRL_CATEGORY: f64 = 1_000.0;
const IDR_CATEGORY: f64 = 3_000_000.0;
const GOLD_CATEGORY: f64 = 0.1; // Troy ounces
const BTC_CATEGORY: f64 = 0.004;
const SPECIAL_CATEGORY: f64 = 200.0;

pub fn threshold_for_symbol(symbol: &str) -> Option<f64> {
    let symbol_upper = symbol.to_ascii_uppercase();
    let symbol = symbol_upper.as_str();

    if matches!(symbol, "ETH" | "WETH" | "CETH" | "STETH" | "CBETH" | "RETH") {
        return Some(ETH_CATEGORY);
    }

    if matches!(
        symbol,
        "USDC" | "USDT" | "DAI" | "BUSD" | "FRAX" | "LUSD" | "TUSD" | "USDP" | "GUSD" | "PYUSD"
    ) {
        return Some(USD_CATEGORY);
    }

    if matches!(
        symbol,
        "FDUSD"
            | "FEI"
            | "GHO"
            | "MIM"
            | "MKUSD"
            | "OUSD"
            | "SUSD"
            | "USD0"
            | "USD1"
            | "USDS"
            | "USDD"
            | "USDE"
            | "XUSD"
            | "ZUSD"
            | "DOLA"
    ) {
        return Some(USD_CATEGORY);
    }

    if matches!(symbol, "EUROC" | "EURCV" | "EURS" | "EURT") {
        return Some(EUR_CATEGORY);
    }

    if matches!(symbol, "GYEN" | "JPYC") {
        return Some(JPY_CATEGORY);
    }

    if symbol == "XSGD" {
        return Some(SGD_CATEGORY);
    }

    if symbol == "CADC" {
        return Some(CAD_CATEGORY);
    }

    if symbol == "BRZ" {
        return Some(BRL_CATEGORY);
    }

    if matches!(symbol, "IDRT" | "XIDR") {
        return Some(IDR_CATEGORY);
    }

    if matches!(symbol, "PAXG" | "XAUT") {
        return Some(GOLD_CATEGORY);
    }

    if symbol == "WBTC" {
        return Some(BTC_CATEGORY);
    }

    if symbol == "RAI" {
        return Some(SPECIAL_CATEGORY);
    }

    None
}
