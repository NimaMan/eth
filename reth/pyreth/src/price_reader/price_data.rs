use eth_prices::{
    AggregatorPriceSource, AmmPriceSource, LiquidityMetrics, OraclePriceSource,
    PriceData as RustPriceData, PriceSource, SimulationPriceSource, Token,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Python wrapper for price data from any source
#[pyclass]
#[derive(Clone)]
pub struct PyPriceData {
    #[pyo3(get)]
    pub pair: String,
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub price_numerator: String,
    #[pyo3(get)]
    pub price_denominator: String,
    #[pyo3(get)]
    pub inverse_price: f64,
    #[pyo3(get)]
    pub inverse_price_numerator: String,
    #[pyo3(get)]
    pub inverse_price_denominator: String,
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
        let price = data.price_as_f64();
        let inverse_price = data.inverse_price_as_f64();

        Ok(PyPriceData {
            pair: data.pair.label.clone(),
            price,
            price_numerator: data.price.numerator.to_string(),
            price_denominator: data.price.denominator.to_string(),
            inverse_price,
            inverse_price_numerator: data.inverse_price.numerator.to_string(),
            inverse_price_denominator: data.inverse_price.denominator.to_string(),
            decimals: data.pair.quote.decimals,
            block_number: data.block_number,
            timestamp: data.timestamp,
            source: data.source.protocol().to_string(),
            source_info,
        })
    }

    /// Parse PriceSource enum into structured Python dictionary
    fn parse_price_source(source: &PriceSource, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);

        match source {
            PriceSource::Amm(data) => Self::parse_amm_source(py, data, &dict)?,
            PriceSource::Oracle(data) => Self::parse_oracle_source(py, data, &dict)?,
            PriceSource::Aggregator(data) => Self::parse_aggregator_source(py, data, &dict)?,
            PriceSource::Simulation(data) => Self::parse_simulation_source(py, data, &dict)?,
        }

        Ok(dict.into())
    }

    fn parse_amm_source(py: Python, data: &AmmPriceSource, dict: &Bound<'_, PyDict>) -> PyResult<()> {
        dict.set_item("protocol", data.protocol.as_str())?;
        dict.set_item("price_id", data.price_id.to_string())?;
        dict.set_item(
            "pool_address",
            format!("0x{}", hex::encode(data.pool.address.as_slice())),
        )?;
        dict.set_item("token0", data.pool.token0.symbol.clone())?;
        dict.set_item("token1", data.pool.token1.symbol.clone())?;
        dict.set_item("token0_info", Self::token_to_dict(py, &data.pool.token0)?)?;
        dict.set_item("token1_info", Self::token_to_dict(py, &data.pool.token1)?)?;
        dict.set_item("fee_bps", data.pool.kind.fee_bps())?;
        dict.set_item("liquidity_tier", data.liquidity.liquidity_tier)?;
        dict.set_item("raw_liquidity", data.liquidity.raw_liquidity.to_string())?;
        let stablecoin =
            Self::identify_stablecoin_from_tokens(&data.pool.token0, &data.pool.token1);
        dict.set_item("stablecoin", stablecoin)?;
        Ok(())
    }

    fn parse_oracle_source(_: Python, data: &OraclePriceSource, dict: &Bound<'_, PyDict>) -> PyResult<()> {
        dict.set_item("protocol", data.price_id.protocol.as_str())?;
        dict.set_item("price_id", data.price_id.to_string())?;
        dict.set_item(
            "feed_address",
            format!("0x{}", hex::encode(data.feed_address)),
        )?;
        dict.set_item("answer", data.answer.to_string())?;
        dict.set_item("answer_decimals", data.answer_decimals)?;
        dict.set_item(
            "round_id",
            data.round_id
                .map(|r| format!("{:#x}", r))
                .unwrap_or_else(|| "None".into()),
        )?;
        dict.set_item("updated_at", data.updated_at)?;
        Ok(())
    }

    fn parse_aggregator_source(
        py: Python,
        data: &AggregatorPriceSource,
        dict: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        dict.set_item("protocol", data.protocol.as_str())?;
        dict.set_item("routes", data.routes.clone())?;
        dict.set_item("amount_in", data.amount_in.to_string())?;
        dict.set_item("amount_out", data.amount_out.to_string())?;
        if let Some(estimated) = data.estimated_gas {
            dict.set_item("estimated_gas", estimated.to_string())?;
        }
        dict.set_item("from_token", data.from_token.symbol.clone())?;
        dict.set_item("to_token", data.to_token.symbol.clone())?;
        dict.set_item(
            "from_token_info",
            Self::token_to_dict(py, &data.from_token)?,
        )?;
        dict.set_item("to_token_info", Self::token_to_dict(py, &data.to_token)?)?;
        Ok(())
    }

    fn parse_simulation_source(
        py: Python,
        data: &SimulationPriceSource,
        dict: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        dict.set_item("protocol", data.price_id.protocol.as_str())?;
        dict.set_item("price_id", data.price_id.to_string())?;
        dict.set_item("route", data.route.clone())?;
        dict.set_item("direction", format!("{:?}", data.direction))?;
        dict.set_item("account", format!("0x{}", hex::encode(data.account)))?;
        dict.set_item("input_token", Self::token_to_dict(py, &data.input_token)?)?;
        dict.set_item("output_token", Self::token_to_dict(py, &data.output_token)?)?;
        dict.set_item("input_amount", data.input_amount.to_string())?;
        dict.set_item("output_amount", data.output_amount.to_string())?;
        if let Some(tax) = data.tax_percent {
            dict.set_item("tax_percent", tax)?;
        }
        if let Some(liq) = &data.liquidity {
            dict.set_item("liquidity", Self::liquidity_to_dict(py, liq)?)?;
        }
        Ok(())
    }

    fn token_to_dict(py: Python, token: &Token) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item(
            "address",
            format!("0x{}", hex::encode(token.address.as_slice())),
        )?;
        dict.set_item("symbol", token.symbol.clone())?;
        dict.set_item("decimals", token.decimals)?;
        Ok(dict.into())
    }

    fn liquidity_to_dict(py: Python, data: &LiquidityMetrics) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("raw_liquidity", data.raw_liquidity.to_string())?;
        dict.set_item("liquidity_tier", data.liquidity_tier)?;
        dict.set_item("significance_multiplier", data.significance_multiplier)?;
        dict.set_item("protocol", data.protocol.as_str())?;
        Ok(dict.into())
    }

    fn identify_stablecoin_from_tokens(token0: &Token, token1: &Token) -> &'static str {
        Self::classify_stable_symbol(&token0.symbol)
            .or_else(|| Self::classify_stable_symbol(&token1.symbol))
            .unwrap_or("Unknown")
    }

    fn classify_stable_symbol(symbol: &str) -> Option<&'static str> {
        macro_rules! match_symbol {
            ($sym:literal, $value:literal) => {
                if symbol.eq_ignore_ascii_case($sym) {
                    return Some($value);
                }
            };
        }

        match_symbol!("USDC", "USDC");
        match_symbol!("USDT", "USDT");
        match_symbol!("DAI", "DAI");
        match_symbol!("FRAX", "FRAX");
        match_symbol!("LUSD", "LUSD");
        match_symbol!("USDP", "USDP");
        match_symbol!("BUSD", "BUSD");
        match_symbol!("TUSD", "TUSD");
        match_symbol!("USDD", "USDD");
        match_symbol!("GUSD", "GUSD");
        match_symbol!("PYUSD", "PYUSD");
        None
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
                    price_numerator: "0".to_string(),
                    price_denominator: "1".to_string(),
                    inverse_price: 0.0,
                    inverse_price_numerator: "0".to_string(),
                    inverse_price_denominator: "1".to_string(),
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
        let dict = PyDict::new_bound(py);
        dict.set_item("pair", &self.pair)?;
        dict.set_item("price", self.price)?;
        dict.set_item("price_numerator", &self.price_numerator)?;
        dict.set_item("price_denominator", &self.price_denominator)?;
        dict.set_item("inverse_price", self.inverse_price)?;
        dict.set_item("inverse_price_numerator", &self.inverse_price_numerator)?;
        dict.set_item("inverse_price_denominator", &self.inverse_price_denominator)?;
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
