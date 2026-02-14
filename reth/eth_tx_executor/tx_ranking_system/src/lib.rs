pub mod gas_cache {
    use ethers_core::types::U256;

    #[derive(Debug, Clone)]
    pub struct GasCache {
        _capacity: usize,
    }

    impl GasCache {
        pub fn new(capacity: usize) -> Self {
            Self { _capacity: capacity }
        }
    }

    #[derive(Debug, Clone)]
    pub struct InstantPrices {
        pub base_fee: U256,
        pub urgent: U256,
        pub mev: U256,
    }

    #[derive(Debug, Clone)]
    pub struct PositionRecommendation {
        pub priority_fee: U256,
        pub gas_price: U256,
        pub expected_position: u32,
        pub percentile: f64,
    }

    #[derive(Debug, Clone)]
    pub struct GasAPI {
        _cache: GasCache,
    }

    impl GasAPI {
        pub fn new(cache: GasCache) -> Self {
            Self { _cache: cache }
        }

        pub fn get_instant_prices(&self) -> InstantPrices {
            InstantPrices {
                base_fee: U256::from(25_000_000_000u64),
                urgent: U256::from(40_000_000_000u64),
                mev: U256::from(55_000_000_000u64),
            }
        }

        pub fn recommend_for_position(&self, position: u32) -> PositionRecommendation {
            let base = self.get_instant_prices().base_fee;
            let bump = U256::from((position.max(1) as u64).saturating_mul(100_000_000u64));
            PositionRecommendation {
                priority_fee: bump,
                gas_price: base + bump,
                expected_position: position,
                percentile: if position <= 1 {
                    99.0
                } else if position <= 10 {
                    90.0
                } else if position <= 50 {
                    70.0
                } else {
                    50.0
                },
            }
        }

        pub fn beats_percentile(&self, gas_price: U256, percentile: f64) -> bool {
            let cutoff = if percentile >= 99.0 {
                U256::from(60_000_000_000u64)
            } else if percentile >= 90.0 {
                U256::from(45_000_000_000u64)
            } else if percentile >= 70.0 {
                U256::from(35_000_000_000u64)
            } else {
                U256::from(25_000_000_000u64)
            };
            gas_price >= cutoff
        }
    }

    #[derive(Debug, Clone)]
    pub struct BlockConsumer {
        _rabbitmq_url: String,
        _cache: GasCache,
        _rpc_url: String,
    }

    impl BlockConsumer {
        pub async fn new(rabbitmq_url: &str, cache: GasCache, rpc_url: &str) -> eyre::Result<Self> {
            Ok(Self {
                _rabbitmq_url: rabbitmq_url.to_string(),
                _cache: cache,
                _rpc_url: rpc_url.to_string(),
            })
        }

        pub async fn start_consuming(self) -> eyre::Result<()> {
            Ok(())
        }
    }
}
