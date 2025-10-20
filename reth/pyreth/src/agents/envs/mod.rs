#[cfg(feature = "prices")]
pub mod stablecoin_env;

#[cfg(not(feature = "prices"))]
pub mod stablecoin_env {
    use pyo3::prelude::*;

    #[pyclass]
    #[derive(Clone, Default)]
    pub struct PyChainSnapshot {
        #[pyo3(get)]
        pub block: u64,
        #[pyo3(get)]
        pub base_fee_wei: u128,
    }

    #[pyclass]
    #[derive(Clone, Default)]
    pub struct PyPortfolioState {
        #[pyo3(get)]
        pub eth_wei: String,
        #[pyo3(get)]
        pub usdc_raw: String,
        #[pyo3(get)]
        pub usdt_raw: String,
        #[pyo3(get)]
        pub dai_raw: String,
    }

    #[pyclass]
    #[derive(Clone, Default)]
    pub struct PyStepOutput {
        #[pyo3(get)]
        pub chain: PyChainSnapshot,
        #[pyo3(get)]
        pub portfolio: PyPortfolioState,
        #[pyo3(get)]
        pub reward: f64,
        #[pyo3(get)]
        pub info: String,
    }

    #[pyclass]
    #[derive(Clone, Default)]
    pub struct PyStablecoinAction;

    #[pymethods]
    impl PyStablecoinAction {
        #[new]
        fn new(
            _args: &pyo3::types::PyTuple,
            _kwargs: Option<&pyo3::types::PyDict>,
        ) -> PyResult<Self> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }
    }

    #[pyclass]
    pub struct PyStablecoinEnv;

    #[pymethods]
    impl PyStablecoinEnv {
        #[new]
        fn new(
            _args: &pyo3::types::PyTuple,
            _kwargs: Option<&pyo3::types::PyDict>,
        ) -> PyResult<Self> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }

        fn default_discrete_labels(
            &self,
            _include_sells: bool,
            _include_noop: bool,
        ) -> PyResult<Vec<String>> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }

        fn build_default_discrete_actions(
            &self,
            _include_sells: bool,
            _include_noop: bool,
            _fixed_buy_wei: Option<u128>,
        ) -> PyResult<Vec<PyStablecoinAction>> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }

        fn step(&mut self, _action: &PyStablecoinAction) -> PyResult<PyStepOutput> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }

        fn state(&self) -> PyResult<(PyChainSnapshot, PyPortfolioState)> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }

        fn enable_cache_default_routes(&mut self, _capacity: usize) -> PyResult<()> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }

        fn prefetch_blocks(&mut self, _start_block: u64, _end_block: u64) -> PyResult<()> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }

        fn get_cached_mean_median(
            &self,
            _block: u64,
        ) -> PyResult<(Option<f64>, Option<f64>, Option<f64>)> {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "stablecoin env requires pyreth `prices` feature",
            ))
        }
    }
}
