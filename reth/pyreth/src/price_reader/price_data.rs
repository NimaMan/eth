use eth_prices::{PriceData as RustPriceData, PriceSource};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

/// Python wrapper for price data from any source
#[pyclass]
#[derive(Clone)]
pub struct PyPriceData {
    #[pyo3(get)]
    pub pair: String,
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub decimals: u8,
    #[pyo3(get)]
    pub block_number: u64,
    #[pyo3(get)]
    pub timestamp: u64,
    #[pyo3(get)]
    pub source: String,
    #[pyo3(get)]
    pub source_info: PyObject,
}

impl PyPriceData {
    /// Create from RustPriceData with Python context for source_info
    pub fn from_rust_data(data: RustPriceData, py: Python) -> PyResult<Self> {
        let source_info = Self::parse_price_source(&data.source, py)?;

        Ok(PyPriceData {
            pair: data.pair,
            price: data.price,
            decimals: data.decimals,
            block_number: data.block_number,
            timestamp: data.timestamp,
            source: format!("{:?}", data.source), // Keep backward compatibility
            source_info,
        })
    }

    /// Parse PriceSource enum into structured Python dictionary
    fn parse_price_source(source: &PriceSource, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);

        match source {
            PriceSource::UniswapV2 {
                pool_address,
                reserve0,
                reserve1,
            } => {
                dict.set_item("protocol", "UniswapV2")?;
                dict.set_item("pool_address", pool_address)?;
                dict.set_item("reserve0", *reserve0)?;
                dict.set_item("reserve1", *reserve1)?;

                // Determine stablecoin from pool address
                let stablecoin = Self::identify_stablecoin_from_pool(pool_address);
                dict.set_item("stablecoin", stablecoin)?;

                // Determine tokens from known pools
                let (token0, token1) = Self::identify_tokens_from_pool(pool_address);
                dict.set_item("token0", token0)?;
                dict.set_item("token1", token1)?;
            }

            PriceSource::UniswapV3 {
                pool_address,
                tick,
                sqrt_price_x96,
                fee_tier,
                liquidity,
            } => {
                dict.set_item("protocol", "UniswapV3")?;
                dict.set_item("pool_address", pool_address)?;
                dict.set_item("tick", *tick)?;
                dict.set_item("sqrt_price_x96", sqrt_price_x96)?;
                dict.set_item("fee_tier", *fee_tier)?;
                dict.set_item("liquidity", liquidity)?;

                let stablecoin = Self::identify_stablecoin_from_pool(pool_address);
                dict.set_item("stablecoin", stablecoin)?;

                let (token0, token1) = Self::identify_tokens_from_pool(pool_address);
                dict.set_item("token0", token0)?;
                dict.set_item("token1", token1)?;
            }

            PriceSource::SushiSwap {
                pool_address,
                reserve0,
                reserve1,
            } => {
                dict.set_item("protocol", "SushiSwap")?;
                dict.set_item("pool_address", pool_address)?;
                dict.set_item("reserve0", *reserve0)?;
                dict.set_item("reserve1", *reserve1)?;

                let stablecoin = Self::identify_stablecoin_from_pool(pool_address);
                dict.set_item("stablecoin", stablecoin)?;

                let (token0, token1) = Self::identify_tokens_from_pool(pool_address);
                dict.set_item("token0", token0)?;
                dict.set_item("token1", token1)?;
            }

            PriceSource::Curve {
                pool_address,
                pool_type,
                balances,
            } => {
                dict.set_item("protocol", "Curve")?;
                dict.set_item("pool_address", pool_address)?;
                dict.set_item("pool_type", pool_type)?;
                dict.set_item("balances", balances.clone())?;

                let stablecoin = Self::identify_stablecoin_from_pool(pool_address);
                dict.set_item("stablecoin", stablecoin)?;
            }

            PriceSource::Balancer {
                pool_address,
                pool_type,
                balances,
                weights,
            } => {
                dict.set_item("protocol", "Balancer")?;
                dict.set_item("pool_address", pool_address)?;
                dict.set_item("pool_type", pool_type)?;
                dict.set_item("balances", balances.clone())?;
                dict.set_item("weights", weights.clone())?;

                let stablecoin = Self::identify_stablecoin_from_pool(pool_address);
                dict.set_item("stablecoin", stablecoin)?;
            }

            PriceSource::Chainlink {
                round_id,
                updated_at,
            } => {
                dict.set_item("protocol", "Chainlink")?;
                dict.set_item("round_id", *round_id)?;
                dict.set_item("updated_at", *updated_at)?;
                dict.set_item("stablecoin", "USD")?; // Chainlink feeds are in USD
            }

            _ => {
                // For other protocols, provide basic info
                dict.set_item("protocol", "Unknown")?;
                dict.set_item("stablecoin", "Unknown")?;
            }
        }

        Ok(dict.into())
    }

    /// Identify stablecoin from known pool addresses
    fn identify_stablecoin_from_pool(pool_address: &str) -> &'static str {
        match pool_address.to_lowercase().as_str() {
            // Uniswap V2 USDC/WETH
            "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc" => "USDC",

            // Uniswap V3 USDC/WETH pools
            "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640" => "USDC", // 0.05%
            "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8" => "USDC", // 0.3%

            // DAI/USDC V3
            "0x5777d92f208679db4b9778590fa3cab3ac9e2168" => "DAI",

            // Add more known pools as needed
            _ => {
                // Try to infer from address patterns or return Unknown
                if pool_address.contains("usdc") || pool_address.contains("USDC") {
                    "USDC"
                } else if pool_address.contains("usdt") || pool_address.contains("USDT") {
                    "USDT"
                } else if pool_address.contains("dai") || pool_address.contains("DAI") {
                    "DAI"
                } else {
                    "Unknown"
                }
            }
        }
    }

    /// Identify token pair from known pool addresses
    fn identify_tokens_from_pool(pool_address: &str) -> (&'static str, &'static str) {
        match pool_address.to_lowercase().as_str() {
            // Uniswap V2 USDC/WETH
            "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc" => ("USDC", "WETH"),

            // Uniswap V3 USDC/WETH pools
            "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640" => ("USDC", "WETH"), // 0.05%
            "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8" => ("USDC", "WETH"), // 0.3%

            // DAI/USDC V3
            "0x5777d92f208679db4b9778590fa3cab3ac9e2168" => ("DAI", "USDC"),

            // Default to unknown
            _ => ("Unknown", "Unknown"),
        }
    }
}

// Keep backward compatibility with automatic conversion
impl From<RustPriceData> for PyPriceData {
    fn from(data: RustPriceData) -> Self {
        // This creates a basic version without Python context
        // For full functionality, use PyPriceData::from_rust_data
        Python::with_gil(|py| {
            PyPriceData::from_rust_data(data, py).unwrap_or_else(|_| {
                // Fallback if parsing fails
                PyPriceData {
                    pair: "Unknown".to_string(),
                    price: 0.0,
                    decimals: 0,
                    block_number: 0,
                    timestamp: 0,
                    source: "Error".to_string(),
                    source_info: py.None(),
                }
            })
        })
    }
}

#[pymethods]
impl PyPriceData {
    /// String representation of the price data with source info
    fn __repr__(&self) -> String {
        Python::with_gil(|py| {
            if let Ok(source_dict) = self.source_info.extract::<&PyDict>(py) {
                let protocol = source_dict
                    .get_item("protocol")
                    .ok()
                    .flatten()
                    .and_then(|p| p.extract::<String>().ok())
                    .unwrap_or_else(|| "Unknown".to_string());
                let stablecoin = source_dict
                    .get_item("stablecoin")
                    .ok()
                    .flatten()
                    .and_then(|s| s.extract::<String>().ok())
                    .unwrap_or_else(|| "Unknown".to_string());

                format!(
                    "PriceData(pair='{}', price={:.6}, protocol='{}', stablecoin='{}', block={})",
                    self.pair, self.price, protocol, stablecoin, self.block_number
                )
            } else {
                format!(
                    "PriceData(pair='{}', price={:.6}, block={}, source='{}')",
                    self.pair, self.price, self.block_number, self.source
                )
            }
        })
    }

    /// Convert to dictionary with structured source info
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("pair", &self.pair)?;
        dict.set_item("price", self.price)?;
        dict.set_item("decimals", self.decimals)?;
        dict.set_item("block_number", self.block_number)?;
        dict.set_item("timestamp", self.timestamp)?;
        dict.set_item("source", &self.source)?; // Keep legacy field
        dict.set_item("source_info", &self.source_info)?;

        Ok(dict.into())
    }

    /// Get protocol name from source_info
    fn get_protocol(&self, py: Python) -> PyResult<String> {
        if let Ok(source_dict) = self.source_info.extract::<&PyDict>(py) {
            if let Some(item) = source_dict.get_item("protocol")? {
                item.extract::<String>()
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(
                    "protocol not found",
                ))
            }
        } else {
            Ok("Unknown".to_string())
        }
    }

    /// Get stablecoin from source_info
    fn get_stablecoin(&self, py: Python) -> PyResult<String> {
        if let Ok(source_dict) = self.source_info.extract::<&PyDict>(py) {
            if let Some(item) = source_dict.get_item("stablecoin")? {
                item.extract::<String>()
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(
                    "stablecoin not found",
                ))
            }
        } else {
            Ok("Unknown".to_string())
        }
    }

    /// Get pool address from source_info
    fn get_pool_address(&self, py: Python) -> PyResult<Option<String>> {
        if let Ok(source_dict) = self.source_info.extract::<&PyDict>(py) {
            if let Some(item) = source_dict.get_item("pool_address")? {
                Ok(Some(item.extract::<String>()?))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Format price with specified decimal places
    fn format_price(&self, decimals: Option<usize>) -> String {
        let decimals = decimals.unwrap_or(2);
        format!("{:.decimals$}", self.price, decimals = decimals)
    }
}
