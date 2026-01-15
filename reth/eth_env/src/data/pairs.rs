use crate::env_core::EnvError;
use eth_prices::core::PriceData;
use eth_prices::price_readers::snapshot::{SushiSwapReader, UniswapV2Reader, UniswapV3Reader};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum PairProtocol {
    UniswapV2,
    UniswapV3,
    SushiSwap,
}

impl PairProtocol {
    pub fn from_str(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "uniswap_v2" | "uniswapv2" | "v2" => Some(Self::UniswapV2),
            "uniswap_v3" | "uniswapv3" | "v3" => Some(Self::UniswapV3),
            "sushiswap" | "sushi" => Some(Self::SushiSwap),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::UniswapV2 => "uniswap_v2",
            Self::UniswapV3 => "uniswap_v3",
            Self::SushiSwap => "sushiswap",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PairRequest {
    pub protocol: PairProtocol,
    pub token: String,
    pub denom: String,
    pub fee_tier: Option<u32>,
}

impl PairRequest {
    pub fn pair_label(&self) -> Result<String, EnvError> {
        match self.protocol {
            PairProtocol::UniswapV2 | PairProtocol::SushiSwap => {
                Ok(format!("{}/{}", self.token, self.denom))
            }
            PairProtocol::UniswapV3 => {
                let fee = self.fee_tier.ok_or_else(|| {
                    EnvError::data(format!(
                        "missing fee_tier for UniswapV3 pair {} / {}",
                        self.token, self.denom
                    ))
                })?;
                Ok(format!("{}/{}_V3_{}", self.token, self.denom, fee))
            }
        }
    }
}

pub struct PairSources {
    uniswap_v2: UniswapV2Reader,
    uniswap_v3: UniswapV3Reader,
    sushiswap: SushiSwapReader,
}

impl PairSources {
    pub fn new(
        uniswap_v2: UniswapV2Reader,
        uniswap_v3: UniswapV3Reader,
        sushiswap: SushiSwapReader,
    ) -> Self {
        Self {
            uniswap_v2,
            uniswap_v3,
            sushiswap,
        }
    }

    pub fn prices_at_block(
        &self,
        requests: &[PairRequest],
        block_number: u64,
    ) -> Result<HashMap<String, PriceData>, EnvError> {
        let mut out = HashMap::with_capacity(requests.len());
        for request in requests {
            let label = request.pair_label()?;
            let price = match request.protocol {
                PairProtocol::UniswapV2 => self
                    .uniswap_v2
                    .get_price_at_block(&label, block_number)
                    .map_err(|e| EnvError::data(format!("uniswap_v2 price error: {e}")))?,
                PairProtocol::UniswapV3 => self
                    .uniswap_v3
                    .get_price_at_block(&label, block_number)
                    .map_err(|e| EnvError::data(format!("uniswap_v3 price error: {e}")))?,
                PairProtocol::SushiSwap => self
                    .sushiswap
                    .get_price_at_block(&label, block_number)
                    .map_err(|e| EnvError::data(format!("sushiswap price error: {e}")))?,
            };
            out.insert(label, price);
        }
        Ok(out)
    }
}
