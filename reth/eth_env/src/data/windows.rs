use crate::data::feeds::PriceFeeds;
use crate::data::pairs::PairRequest;
use crate::env_core::EnvError;
use eth_prices::core::PriceData;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct WindowRecord {
    pub start_block: u64,
    pub end_block: u64,
    pub timestamp_start: i64,
    pub timestamp_end: i64,
    pub chainlink_price_start: f64,
    pub chainlink_price_end: f64,
    pub chainlink_return: f64,
    pub base_fee_per_gas: u64,
    pub usdc_mean_mid: f64,
    pub usdc_spread_bps: f64,
    pub usdc_venue_count: usize,
    pub usdt_mean_mid: f64,
    pub usdt_spread_bps: f64,
    pub usdt_venue_count: usize,
    pub dai_mean_mid: f64,
    pub dai_spread_bps: f64,
    pub dai_venue_count: usize,
    pub target_up: bool,
}

pub async fn collect_windows(
    feeds: &PriceFeeds,
    start_block: u64,
    window_count: usize,
    interval_blocks: u64,
    pair_requests: Option<&[PairRequest]>,
) -> Result<Vec<WindowRecord>, EnvError> {
    if interval_blocks == 0 {
        return Err(EnvError::data("interval_blocks must be greater than 0"));
    }

    let mut records = Vec::with_capacity(window_count);
    let mut current_start = start_block;

    for _ in 0..window_count {
        let snapshot = feeds
            .fetch_snapshot_at_block(current_start, pair_requests)
            .await?;
        let end_block = current_start.saturating_add(interval_blocks);
        let chainlink_end = feeds.chainlink_price_at_block(end_block).await?;

        let chainlink_price_start = snapshot.chainlink_price.price_as_f64();
        let chainlink_price_end = chainlink_end.price_as_f64();

        let chainlink_return = if chainlink_price_start != 0.0 {
            (chainlink_price_end - chainlink_price_start) / chainlink_price_start
        } else {
            0.0
        };

        let target_up = chainlink_price_end >= chainlink_price_start;

        let (usdc_mean_mid, usdc_spread_bps, usdc_count) =
            aggregate_prices(&snapshot.eth_usdc_prices);
        let (usdt_mean_mid, usdt_spread_bps, usdt_count) =
            aggregate_prices(&snapshot.eth_usdt_prices);
        let (dai_mean_mid, dai_spread_bps, dai_count) = aggregate_prices(&snapshot.eth_dai_prices);

        records.push(WindowRecord {
            start_block: current_start,
            end_block,
            timestamp_start: snapshot.timestamp_unix,
            timestamp_end: chainlink_end.timestamp as i64,
            chainlink_price_start,
            chainlink_price_end,
            chainlink_return,
            base_fee_per_gas: snapshot.base_fee_per_gas,
            usdc_mean_mid,
            usdc_spread_bps,
            usdc_venue_count: usdc_count,
            usdt_mean_mid,
            usdt_spread_bps,
            usdt_venue_count: usdt_count,
            dai_mean_mid,
            dai_spread_bps,
            dai_venue_count: dai_count,
            target_up,
        });

        current_start = end_block;
    }

    Ok(records)
}

fn aggregate_prices(prices: &HashMap<String, PriceData>) -> (f64, f64, usize) {
    if prices.is_empty() {
        return (f64::NAN, f64::NAN, 0);
    }

    let mids: Vec<f64> = prices.values().map(|p| p.price_as_f64()).collect();
    let count = mids.len();
    let mean = mids.iter().sum::<f64>() / count as f64;
    let variance = mids
        .iter()
        .map(|price| {
            let diff = price - mean;
            diff * diff
        })
        .sum::<f64>()
        / count as f64;
    let std_dev = variance.sqrt();
    let spread_bps = if mean.abs() > f64::EPSILON {
        (std_dev / mean) * 10_000.0
    } else {
        f64::NAN
    };

    (mean, spread_bps, count)
}
