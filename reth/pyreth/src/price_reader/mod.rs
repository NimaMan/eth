/// Python price reader bindings. When the `prices` feature is disabled we
/// expose lightweight placeholder classes so the rest of the module graph
/// continues to compile without depending on the heavy price crates.

#[cfg(feature = "prices")]
pub mod client;
#[cfg(feature = "prices")]
pub mod price_data;

#[cfg(feature = "prices")]
pub use client::PyEthPriceClient;
#[cfg(feature = "prices")]
pub use price_data::PyPriceData;

#[cfg(not(feature = "prices"))]
mod stub {
    use pyo3::prelude::*;
    use std::sync::Arc;
    use tx_simulator::TxSimulator;

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

    impl Default for PyPriceData {
        fn default() -> Self {
            Python::with_gil(|py| PyPriceData {
                pair: String::new(),
                price: 0.0,
                decimals: 0,
                block_number: 0,
                timestamp: 0,
                source: "prices feature disabled".to_string(),
                source_info: py.None(),
            })
        }
    }

    #[pymethods]
    impl PyPriceData {
        #[new]
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[pyclass]
    pub struct PyEthPriceClient;

    impl PyEthPriceClient {
        pub fn from_simulator(_simulator: Arc<TxSimulator>) -> Self {
            PyEthPriceClient
        }
    }

    #[pymethods]
    impl PyEthPriceClient {
        #[new]
        pub fn new() -> Self {
            PyEthPriceClient
        }

        fn __repr__(&self) -> &'static str {
            "PyEthPriceClient(prices feature disabled)"
        }

        #[pyo3(text_signature = "($self, /, *args, **kwargs)")]
        fn get_eth_price<'py>(
            &self,
            _py: Python<'py>,
            _protocol: &str,
            _stablecoin: &str,
        ) -> PyResult<PyPriceData> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "pyreth built without `prices` feature",
            ))
        }
    }
}

#[cfg(not(feature = "prices"))]
pub use stub::{PyEthPriceClient, PyPriceData};
