use std::collections::HashMap;
use std::time::{Duration, Instant};

use eth_price::PriceData;
use tokio::sync::RwLock;

const LATEST_PRICE_TTL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PriceCacheKey {
    pub venue: String,
    pub pair: String,
    pub block: Option<u64>,
}

impl PriceCacheKey {
    pub fn new(venue: impl Into<String>, pair: impl Into<String>, block: Option<u64>) -> Self {
        Self {
            venue: venue.into(),
            pair: pair.into(),
            block,
        }
    }
}

#[derive(Clone)]
struct PriceCacheEntry {
    inserted_at: Instant,
    price: PriceData,
}

#[derive(Default)]
pub struct PriceCache {
    prices: RwLock<HashMap<PriceCacheKey, PriceCacheEntry>>,
}

impl PriceCache {
    pub async fn get(&self, key: &PriceCacheKey) -> Option<PriceData> {
        let prices = self.prices.read().await;
        let entry = prices.get(key)?;
        if key.block.is_none() && entry.inserted_at.elapsed() > LATEST_PRICE_TTL {
            return None;
        }
        Some(entry.price.clone())
    }

    pub async fn insert(&self, key: PriceCacheKey, price: PriceData) {
        let mut prices = self.prices.write().await;
        prices.insert(
            key,
            PriceCacheEntry {
                inserted_at: Instant::now(),
                price,
            },
        );
    }
}
