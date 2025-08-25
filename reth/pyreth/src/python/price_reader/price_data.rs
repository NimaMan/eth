use pyo3::prelude::*;
use eth_price_leverage::PriceData as RustPriceData;
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
}

impl From<RustPriceData> for PyPriceData {
    fn from(data: RustPriceData) -> Self {
        PyPriceData {
            pair: data.pair,
            price: data.price,
            decimals: data.decimals,
            block_number: data.block_number,
            timestamp: data.timestamp,
            source: format!("{:?}", data.source),
        }
    }
}

#[pymethods]
impl PyPriceData {
    /// String representation of the price data
    fn __repr__(&self) -> String {
        format!(
            "PriceData(pair='{}', price={:.6}, block={}, source='{}')",
            self.pair, self.price, self.block_number, self.source
        )
    }
    
    /// Convert to dictionary
    fn to_dict(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        dict.insert("pair".to_string(), self.pair.clone());
        dict.insert("price".to_string(), self.price.to_string());
        dict.insert("decimals".to_string(), self.decimals.to_string());
        dict.insert("block_number".to_string(), self.block_number.to_string());
        dict.insert("timestamp".to_string(), self.timestamp.to_string());
        dict.insert("source".to_string(), self.source.clone());
        dict
    }
    
    /// Format price with specified decimal places
    fn format_price(&self, decimals: Option<usize>) -> String {
        let decimals = decimals.unwrap_or(2);
        format!("{:.decimals$}", self.price, decimals = decimals)
    }
}