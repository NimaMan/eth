use std::collections::HashMap;
use std::sync::Arc;

use crate::core::{PriceData, PriceError};
use crate::dex::{CurveReader, SushiSwapReader, UniswapV2Reader, UniswapV3Reader};
use reth_chain_query::common_addresses::dex_token_denom_pairs::{
    eth_dai_pairs, eth_usdc_pairs, eth_usdt_pairs, StablecoinPairSpec,
};

/// Stablecoin price reader (CONTRACT-BALANCE-BASED) for ETH quoted in USDC, USDT, and DAI across major protocols.
///
/// - Uses direct pools where available (Uniswap V2/V3, SushiSwap, Curve TriCrypto for USDT)
/// - Avoids oracles for stablecoins by default
/// - Returns per-protocol prices as a simple map
pub struct StablecoinPriceReader {
    pub uni_v2: UniswapV2Reader,
    pub uni_v3: UniswapV3Reader,
    pub sushi: SushiSwapReader,
    pub curve: CurveReader,
}

impl StablecoinPriceReader {
    pub fn from_provider(
        provider: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        Ok(Self {
            uni_v2: UniswapV2Reader::from_provider(provider.clone())?,
            uni_v3: UniswapV3Reader::from_provider(provider.clone())?,
            sushi: SushiSwapReader::from_provider(provider.clone())?,
            curve: CurveReader::from_provider(provider)?,
        })
    }

    /// Fetch ETH/USDC across supported protocols.
    pub fn eth_usdc(&self) -> HashMap<String, PriceData> {
        self.prices_for_specs_sync(eth_usdc_pairs())
    }

    /// Fetch ETH/USDC at a specific block height.
    pub fn eth_usdc_at_block(&self, block_number: u64) -> HashMap<String, PriceData> {
        self.prices_for_specs_at_block(eth_usdc_pairs(), block_number)
    }

    /// Fetch ETH/USDT across supported protocols.
    pub async fn eth_usdt(&self) -> HashMap<String, PriceData> {
        let non_curve_specs: Vec<StablecoinPairSpec> = eth_usdt_pairs()
            .iter()
            .filter(|spec| spec.protocol != "Curve")
            .copied()
            .collect();
        let mut out = self.prices_for_specs_sync(&non_curve_specs);

        for spec in eth_usdt_pairs().iter().filter(|s| s.protocol == "Curve") {
            match self.curve.get_latest_price(spec.pair_label).await {
                Ok(price) => {
                    out.insert(spec.key.to_string(), price);
                }
                Err(err) => {
                    tracing::warn!(
                        target: "stablecoin_price_reader",
                        protocol = spec.protocol,
                        pair = spec.pair_label,
                        error = %err,
                        "failed to fetch stablecoin price"
                    );
                }
            }
        }

        out
    }

    /// Fetch ETH/USDT across supported protocols at a specific block height.
    pub async fn eth_usdt_at_block(&self, block_number: u64) -> HashMap<String, PriceData> {
        let non_curve_specs: Vec<StablecoinPairSpec> = eth_usdt_pairs()
            .iter()
            .filter(|spec| spec.protocol != "Curve")
            .copied()
            .collect();
        let mut out = self.prices_for_specs_at_block(&non_curve_specs, block_number);

        for spec in eth_usdt_pairs().iter().filter(|s| s.protocol == "Curve") {
            match self
                .curve
                .get_price_at_block(spec.pair_label, block_number)
                .await
            {
                Ok(price) => {
                    out.insert(spec.key.to_string(), price);
                }
                Err(err) => {
                    tracing::warn!(
                        target: "stablecoin_price_reader",
                        protocol = spec.protocol,
                        pair = spec.pair_label,
                        block = block_number,
                        error = %err,
                        "failed to fetch stablecoin price at block"
                    );
                }
            }
        }

        out
    }

    /// Fetch ETH/DAI directly and derive via USDC if missing for a given protocol group.
    pub fn eth_dai(&self) -> HashMap<String, PriceData> {
        self.prices_for_specs_sync(eth_dai_pairs())
    }

    /// Fetch ETH/DAI at a specific block height.
    pub fn eth_dai_at_block(&self, block_number: u64) -> HashMap<String, PriceData> {
        self.prices_for_specs_at_block(eth_dai_pairs(), block_number)
    }

    /// Convenience: fetch all three maps.
    pub async fn eth_stablecoin_prices(&self) -> HashMap<&'static str, HashMap<String, PriceData>> {
        let mut res = HashMap::new();
        res.insert("USDC", self.eth_usdc());
        res.insert("USDT", self.eth_usdt().await);
        res.insert("DAI", self.eth_dai());
        res
    }

    /// Convenience: fetch all three maps at a specific block height.
    pub async fn eth_stablecoin_prices_at_block(
        &self,
        block_number: u64,
    ) -> HashMap<&'static str, HashMap<String, PriceData>> {
        let mut res = HashMap::new();
        res.insert("USDC", self.eth_usdc_at_block(block_number));
        res.insert("USDT", self.eth_usdt_at_block(block_number).await);
        res.insert("DAI", self.eth_dai_at_block(block_number));
        res
    }

    fn prices_for_specs_sync(&self, specs: &[StablecoinPairSpec]) -> HashMap<String, PriceData> {
        let mut out = HashMap::new();
        for spec in specs {
            if spec.protocol == "Curve" {
                continue;
            }
            match self.fetch_price_sync(spec) {
                Ok(price) => {
                    out.insert(spec.key.to_string(), price);
                }
                Err(err) => {
                    tracing::warn!(
                        target: "stablecoin_price_reader",
                        protocol = spec.protocol,
                        pair = spec.pair_label,
                        error = %err,
                        "failed to fetch stablecoin price"
                    );
                }
            }
        }
        out
    }

    fn prices_for_specs_at_block(
        &self,
        specs: &[StablecoinPairSpec],
        block_number: u64,
    ) -> HashMap<String, PriceData> {
        let mut out = HashMap::new();
        for spec in specs {
            if spec.protocol == "Curve" {
                continue;
            }
            match self.fetch_price_at_block(spec, block_number) {
                Ok(price) => {
                    out.insert(spec.key.to_string(), price);
                }
                Err(err) => {
                    tracing::warn!(
                        target: "stablecoin_price_reader",
                        protocol = spec.protocol,
                        pair = spec.pair_label,
                        block = block_number,
                        error = %err,
                        "failed to fetch stablecoin price at block"
                    );
                }
            }
        }
        out
    }

    fn fetch_price_sync(&self, spec: &StablecoinPairSpec) -> Result<PriceData, PriceError> {
        match spec.protocol {
            "UniswapV2" => self.uni_v2.get_latest_price(spec.pair_label),
            "SushiSwap" => self.sushi.get_latest_price(spec.pair_label),
            "UniswapV3" => self.uni_v3.get_latest_price(spec.pair_label),
            other => Err(PriceError::SourceNotSupported(format!(
                "Protocol {} not supported in StablecoinPriceReader",
                other
            ))),
        }
    }

    fn fetch_price_at_block(
        &self,
        spec: &StablecoinPairSpec,
        block_number: u64,
    ) -> Result<PriceData, PriceError> {
        match spec.protocol {
            "UniswapV2" => self
                .uni_v2
                .get_price_at_block(spec.pair_label, block_number),
            "SushiSwap" => self.sushi.get_price_at_block(spec.pair_label, block_number),
            "UniswapV3" => self
                .uni_v3
                .get_price_at_block(spec.pair_label, block_number),
            other => Err(PriceError::SourceNotSupported(format!(
                "Protocol {} not supported in StablecoinPriceReader at block {}",
                other, block_number
            ))),
        }
    }
}
