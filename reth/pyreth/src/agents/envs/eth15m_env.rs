#[cfg(feature = "prices")]
mod real {
    use crate::price_reader::PyPriceData;
    use eth_env::data::{FeedSnapshot, PriceFeeds};
    use eth_env::{EnvError, Eth15mAction, Eth15mConfig, Eth15mEnv, Eth15mObservation, Eth15mStep};
    use eth_prices::core::PriceData;
    use pyo3::prelude::*;
    use pyo3::types::PyDict;
    use serde_json;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    fn default_reth_datadir() -> String {
        std::env::var("RETH_DATADIR").unwrap_or_else(|_| {
            format!(
                "{}/.local/share/reth/mainnet",
                std::env::var("HOME").unwrap_or_else(|_| ".".into())
            )
        })
    }

    fn map_env_error(err: EnvError) -> PyErr {
        use eth_env::EnvError;
        match err {
            EnvError::Data(msg) => {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("data error: {msg}"))
            }
            EnvError::Simulation(msg) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("simulation error: {msg}"),
            ),
            EnvError::Other(inner) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "environment error: {inner:#}"
            )),
        }
    }

    fn create_runtime() -> PyResult<Runtime> {
        Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "failed to create Tokio runtime: {e}"
            ))
        })
    }

    fn price_map_to_dict(py: Python<'_>, map: HashMap<String, PriceData>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (key, price) in map {
            let py_price = Py::new(py, PyPriceData::from_rust_data(price, py)?)?;
            dict.set_item(key, py_price)?;
        }
        Ok(dict.into())
    }

    fn price_to_py(py: Python<'_>, price: PriceData) -> PyResult<Py<PyPriceData>> {
        Py::new(py, PyPriceData::from_rust_data(price, py)?)
    }

    #[pyclass]
    #[derive(Clone)]
    pub struct PyEth15mObservation {
        #[pyo3(get)]
        pub timestamp_unix: i64,
        #[pyo3(get)]
        pub block_number: u64,
        #[pyo3(get)]
        pub base_fee_per_gas: u64,
        #[pyo3(get)]
        pub chainlink_price: Py<PyPriceData>,
        #[pyo3(get)]
        pub eth_usdc_prices: Py<PyDict>,
        #[pyo3(get)]
        pub eth_usdt_prices: Py<PyDict>,
        #[pyo3(get)]
        pub eth_dai_prices: Py<PyDict>,
    }

    impl PyEth15mObservation {
        fn from_observation(py: Python<'_>, obs: Eth15mObservation) -> PyResult<Self> {
            Ok(Self {
                timestamp_unix: obs.timestamp_unix,
                block_number: obs.block_number,
                base_fee_per_gas: obs.base_fee_per_gas,
                chainlink_price: price_to_py(py, obs.chainlink_price)?,
                eth_usdc_prices: price_map_to_dict(py, obs.eth_usdc_prices)?,
                eth_usdt_prices: price_map_to_dict(py, obs.eth_usdt_prices)?,
                eth_dai_prices: price_map_to_dict(py, obs.eth_dai_prices)?,
            })
        }

        fn from_snapshot(py: Python<'_>, snapshot: FeedSnapshot) -> PyResult<Self> {
            let obs: Eth15mObservation = snapshot.into();
            Self::from_observation(py, obs)
        }
    }

    #[pyclass]
    pub struct PyEth15mStep {
        #[pyo3(get)]
        pub observation: PyEth15mObservation,
        #[pyo3(get)]
        pub reward: f64,
        #[pyo3(get)]
        pub done: bool,
        #[pyo3(get)]
        pub info: String,
    }

    impl PyEth15mStep {
        fn from_step(py: Python<'_>, step: Eth15mStep) -> PyResult<Self> {
            let info = if step.info.is_null() {
                "null".to_string()
            } else {
                serde_json::to_string(&step.info).unwrap_or_else(|_| "null".into())
            };
            Ok(Self {
                observation: PyEth15mObservation::from_observation(py, step.observation)?,
                reward: step.reward,
                done: step.done,
                info,
            })
        }
    }

    #[pyclass]
    #[derive(Clone)]
    pub struct PyEth15mAction {
        pub(crate) inner: Eth15mAction,
    }

    #[pymethods]
    impl PyEth15mAction {
        #[new]
        #[pyo3(signature = (prediction=None))]
        fn new(prediction: Option<f64>) -> Self {
            match prediction {
                Some(price) => Self {
                    inner: Eth15mAction::Predict { price },
                },
                None => Self {
                    inner: Eth15mAction::Hold,
                },
            }
        }

        #[staticmethod]
        fn hold() -> Self {
            Self {
                inner: Eth15mAction::Hold,
            }
        }

        #[staticmethod]
        fn predict(price: f64) -> Self {
            Self {
                inner: Eth15mAction::Predict { price },
            }
        }
    }

    #[pyclass]
    pub struct PyEth15mEnv {
        inner: Eth15mEnv,
        rt: Runtime,
    }

    #[pymethods]
    impl PyEth15mEnv {
        #[new]
        #[pyo3(signature = (reth_datadir=None, dataset_capacity=2048))]
        fn new(reth_datadir: Option<String>, dataset_capacity: usize) -> PyResult<Self> {
            let mut config = Eth15mConfig::default();
            if let Some(path) = reth_datadir {
                config.reth_datadir = path;
            } else {
                config.reth_datadir = default_reth_datadir();
            }
            config.dataset_capacity = dataset_capacity;
            let env = Eth15mEnv::new(config).map_err(map_env_error)?;
            let rt = create_runtime()?;
            Ok(Self { inner: env, rt })
        }

        fn reset(&mut self, py: Python<'_>) -> PyResult<PyEth15mObservation> {
            let obs = self
                .rt
                .block_on(self.inner.reset())
                .map_err(map_env_error)?;
            PyEth15mObservation::from_observation(py, obs)
        }

        fn step(&mut self, py: Python<'_>, action: &PyEth15mAction) -> PyResult<PyEth15mStep> {
            let step = self
                .rt
                .block_on(self.inner.step(action.inner.clone()))
                .map_err(map_env_error)?;
            PyEth15mStep::from_step(py, step)
        }

        fn latest_block(&self) -> PyResult<u64> {
            self.inner.latest_block_number().map_err(map_env_error)
        }

        fn latest_snapshot(&self, py: Python<'_>) -> PyResult<PyEth15mObservation> {
            let obs = self
                .rt
                .block_on(self.inner.latest_observation())
                .map_err(map_env_error)?;
            PyEth15mObservation::from_observation(py, obs)
        }

        fn snapshot_at_block(
            &self,
            py: Python<'_>,
            block_number: u64,
        ) -> PyResult<PyEth15mObservation> {
            let obs = self
                .rt
                .block_on(self.inner.observation_at_block(block_number))
                .map_err(map_env_error)?;
            PyEth15mObservation::from_observation(py, obs)
        }
    }

    #[pyclass]
    pub struct PyEth15mFeeds {
        feeds: PriceFeeds,
        rt: Runtime,
    }

    #[pymethods]
    impl PyEth15mFeeds {
        #[new]
        fn new(reth_datadir: Option<String>) -> PyResult<Self> {
            let datadir = reth_datadir.unwrap_or_else(default_reth_datadir);
            let feeds = PriceFeeds::new(&datadir).map_err(map_env_error)?;
            let rt = create_runtime()?;
            Ok(Self { feeds, rt })
        }

        fn latest_block(&self) -> PyResult<u64> {
            self.feeds.latest_block_number().map_err(map_env_error)
        }

        fn latest_snapshot(&self, py: Python<'_>) -> PyResult<PyEth15mObservation> {
            let snapshot = self
                .rt
                .block_on(self.feeds.fetch_snapshot())
                .map_err(map_env_error)?;
            PyEth15mObservation::from_snapshot(py, snapshot)
        }

        fn snapshot_at_block(
            &self,
            py: Python<'_>,
            block_number: u64,
        ) -> PyResult<PyEth15mObservation> {
            let snapshot = self
                .rt
                .block_on(self.feeds.fetch_snapshot_at_block(block_number))
                .map_err(map_env_error)?;
            PyEth15mObservation::from_snapshot(py, snapshot)
        }

        #[pyo3(signature = (start_block, count, step=1))]
        fn snapshot_range(
            &self,
            py: Python<'_>,
            start_block: u64,
            count: usize,
            step: u64,
        ) -> PyResult<Vec<PyEth15mObservation>> {
            if step == 0 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "step must be at least 1",
                ));
            }
            let mut block = start_block;
            let mut out = Vec::with_capacity(count);
            for _ in 0..count {
                let snapshot = self
                    .rt
                    .block_on(self.feeds.fetch_snapshot_at_block(block))
                    .map_err(map_env_error)?;
                out.push(PyEth15mObservation::from_snapshot(py, snapshot)?);
                block = block.saturating_sub(step);
            }
            Ok(out)
        }
    }
}

#[cfg(feature = "prices")]
pub use real::*;

#[cfg(not(feature = "prices"))]
#[allow(non_local_definitions)]
mod stub {
    use pyo3::prelude::*;

    #[pyclass]
    #[derive(Clone)]
    pub struct PyEth15mObservation;

    #[pyclass]
    pub struct PyEth15mStep;

    #[pyclass]
    #[derive(Clone)]
    pub struct PyEth15mAction;

    #[pyclass]
    pub struct PyEth15mEnv;

    #[pyclass]
    pub struct PyEth15mFeeds;

    macro_rules! disabled {
        ($msg:expr) => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>($msg)
        };
    }

    #[pymethods]
    impl PyEth15mObservation {
        #[new]
        fn new() -> PyResult<Self> {
            Err(disabled!("pyreth built without `prices` feature"))
        }
    }

    #[pymethods]
    impl PyEth15mStep {
        #[new]
        fn new() -> PyResult<Self> {
            Err(disabled!("pyreth built without `prices` feature"))
        }
    }

    #[pymethods]
    impl PyEth15mAction {
        #[new]
        fn new() -> PyResult<Self> {
            Err(disabled!("pyreth built without `prices` feature"))
        }
    }

    #[pymethods]
    impl PyEth15mEnv {
        #[new]
        #[pyo3(signature = (_reth_datadir=None, _dataset_capacity=2048))]
        fn new(_reth_datadir: Option<String>, _dataset_capacity: usize) -> PyResult<Self> {
            Err(disabled!("pyreth built without `prices` feature"))
        }
    }

    #[pymethods]
    impl PyEth15mFeeds {
        #[new]
        fn new(_reth_datadir: Option<String>) -> PyResult<Self> {
            Err(disabled!("pyreth built without `prices` feature"))
        }
    }
}

#[cfg(not(feature = "prices"))]
pub use stub::*;
