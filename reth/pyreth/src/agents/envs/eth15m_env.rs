#[cfg(feature = "prices")]
mod real {
    use crate::price_reader::PyPriceData;
    use eth_env::data::pairs::PairProtocol;
    use eth_env::data::{windows, FeedSnapshot, PairRequest, PriceFeeds};
    use eth_env::EnvError;
    use eth_prices::core::PriceData;
    use pyo3::prelude::*;
    use pyo3::types::PyDict;
    use pyo3::FromPyObject;
    use std::collections::HashMap;
    use std::str::FromStr;
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

    #[derive(FromPyObject)]
    struct PyPairRequest {
        protocol: String,
        token: String,
        denom: Option<String>,
        fee: Option<u32>,
    }

    fn convert_pair_requests(
        requests: Option<Vec<PyPairRequest>>,
    ) -> PyResult<Option<Vec<PairRequest>>> {
        if let Some(reqs) = requests {
            let mut out = Vec::with_capacity(reqs.len());
            for req in reqs {
                let protocol = PairProtocol::from_str(&req.protocol).ok_or_else(|| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "unknown pair protocol '{}'",
                        req.protocol
                    ))
                })?;
                let denom = req.denom.unwrap_or_else(|| "WETH".to_string());
                out.push(PairRequest {
                    protocol,
                    token: req.token.to_uppercase(),
                    denom: denom.to_uppercase(),
                    fee_tier: req.fee,
                });
            }
            Ok(Some(out))
        } else {
            Ok(None)
        }
    }

    #[pyclass]
    #[derive(Clone)]
    pub struct PyWindowRecord {
        #[pyo3(get)]
        pub start_block: u64,
        #[pyo3(get)]
        pub end_block: u64,
        #[pyo3(get)]
        pub timestamp_start: i64,
        #[pyo3(get)]
        pub timestamp_end: i64,
        #[pyo3(get)]
        pub chainlink_price_start: f64,
        #[pyo3(get)]
        pub chainlink_price_end: f64,
        #[pyo3(get)]
        pub chainlink_return: f64,
        #[pyo3(get)]
        pub base_fee_per_gas: u64,
        #[pyo3(get)]
        pub usdc_mean_mid: f64,
        #[pyo3(get)]
        pub usdc_spread_bps: f64,
        #[pyo3(get)]
        pub usdc_venue_count: usize,
        #[pyo3(get)]
        pub usdt_mean_mid: f64,
        #[pyo3(get)]
        pub usdt_spread_bps: f64,
        #[pyo3(get)]
        pub usdt_venue_count: usize,
        #[pyo3(get)]
        pub dai_mean_mid: f64,
        #[pyo3(get)]
        pub dai_spread_bps: f64,
        #[pyo3(get)]
        pub dai_venue_count: usize,
        #[pyo3(get)]
        pub target_up: bool,
    }

    impl From<windows::WindowRecord> for PyWindowRecord {
        fn from(record: windows::WindowRecord) -> Self {
            Self {
                start_block: record.start_block,
                end_block: record.end_block,
                timestamp_start: record.timestamp_start,
                timestamp_end: record.timestamp_end,
                chainlink_price_start: record.chainlink_price_start,
                chainlink_price_end: record.chainlink_price_end,
                chainlink_return: record.chainlink_return,
                base_fee_per_gas: record.base_fee_per_gas,
                usdc_mean_mid: record.usdc_mean_mid,
                usdc_spread_bps: record.usdc_spread_bps,
                usdc_venue_count: record.usdc_venue_count,
                usdt_mean_mid: record.usdt_mean_mid,
                usdt_spread_bps: record.usdt_spread_bps,
                usdt_venue_count: record.usdt_venue_count,
                dai_mean_mid: record.dai_mean_mid,
                dai_spread_bps: record.dai_spread_bps,
                dai_venue_count: record.dai_venue_count,
                target_up: record.target_up,
            }
        }
    }

    #[pyclass]
    #[derive(Clone)]
    pub struct PyPolymarketTarget {
        #[pyo3(get)]
        pub window_index: usize,
        #[pyo3(get)]
        pub window_start_timestamp: i64,
        #[pyo3(get)]
        pub window_end_timestamp: i64,
        #[pyo3(get)]
        pub start_block: u64,
        #[pyo3(get)]
        pub end_block: u64,
    }

    impl PyPolymarketTarget {
        fn new(
            window_index: usize,
            window_start_timestamp: i64,
            window_end_timestamp: i64,
            start_block: u64,
            end_block: u64,
        ) -> Self {
            Self {
                window_index,
                window_start_timestamp,
                window_end_timestamp,
                start_block,
                end_block,
            }
        }
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
        #[pyo3(get)]
        pub pair_prices: Py<PyDict>,
    }

    impl PyEth15mObservation {
        fn from_snapshot(py: Python<'_>, snapshot: FeedSnapshot) -> PyResult<Self> {
            Ok(Self {
                timestamp_unix: snapshot.timestamp_unix,
                block_number: snapshot.block_number,
                base_fee_per_gas: snapshot.base_fee_per_gas,
                chainlink_price: price_to_py(py, snapshot.chainlink_price)?,
                eth_usdc_prices: price_map_to_dict(py, snapshot.eth_usdc_prices)?,
                eth_usdt_prices: price_map_to_dict(py, snapshot.eth_usdt_prices)?,
                eth_dai_prices: price_map_to_dict(py, snapshot.eth_dai_prices)?,
                pair_prices: price_map_to_dict(py, snapshot.pair_prices)?,
            })
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

        #[pyo3(signature = (pair_requests=None))]
        fn latest_snapshot(
            &self,
            py: Python<'_>,
            pair_requests: Option<Vec<PyPairRequest>>,
        ) -> PyResult<PyEth15mObservation> {
            let reqs = convert_pair_requests(pair_requests)?;
            let snapshot = self
                .rt
                .block_on(self.feeds.fetch_snapshot(reqs.as_deref()))
                .map_err(map_env_error)?;
            PyEth15mObservation::from_snapshot(py, snapshot)
        }

        fn snapshot_at_block(
            &self,
            py: Python<'_>,
            block_number: u64,
            pair_requests: Option<Vec<PyPairRequest>>,
        ) -> PyResult<PyEth15mObservation> {
            let reqs = convert_pair_requests(pair_requests)?;
            let snapshot = self
                .rt
                .block_on(
                    self.feeds
                        .fetch_snapshot_at_block(block_number, reqs.as_deref()),
                )
                .map_err(map_env_error)?;
            PyEth15mObservation::from_snapshot(py, snapshot)
        }

        #[pyo3(signature = (start_block, count, step=1, pair_requests=None))]
        fn snapshot_range(
            &self,
            py: Python<'_>,
            start_block: u64,
            count: usize,
            step: u64,
            pair_requests: Option<Vec<PyPairRequest>>,
        ) -> PyResult<Vec<PyEth15mObservation>> {
            if step == 0 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "step must be at least 1",
                ));
            }
            let reqs = convert_pair_requests(pair_requests)?;
            let mut block = start_block;
            let mut out = Vec::with_capacity(count);
            for _ in 0..count {
                let snapshot = self
                    .rt
                    .block_on(self.feeds.fetch_snapshot_at_block(block, reqs.as_deref()))
                    .map_err(map_env_error)?;
                out.push(PyEth15mObservation::from_snapshot(py, snapshot)?);
                block = block.saturating_sub(step);
            }
            Ok(out)
        }

        #[pyo3(signature = (start_block, window_count, interval_blocks, pair_requests=None))]
        fn export_windows(
            &self,
            start_block: u64,
            window_count: usize,
            interval_blocks: u64,
            pair_requests: Option<Vec<PyPairRequest>>,
        ) -> PyResult<Vec<PyWindowRecord>> {
            let reqs = convert_pair_requests(pair_requests)?;
            let records = self
                .rt
                .block_on(windows::collect_windows(
                    &self.feeds,
                    start_block,
                    window_count,
                    interval_blocks,
                    reqs.as_deref(),
                ))
                .map_err(map_env_error)?;
            Ok(records.into_iter().map(PyWindowRecord::from).collect())
        }

        fn block_at_timestamp(&self, timestamp_unix: i64) -> PyResult<u64> {
            self.rt
                .block_on(self.feeds.block_at_timestamp(timestamp_unix))
                .map_err(map_env_error)
        }

        #[pyo3(signature = (timestamp_unix, pair_requests=None))]
        fn snapshot_at_timestamp(
            &self,
            py: Python<'_>,
            timestamp_unix: i64,
            pair_requests: Option<Vec<PyPairRequest>>,
        ) -> PyResult<PyEth15mObservation> {
            let reqs = convert_pair_requests(pair_requests)?;
            let snapshot = self
                .rt
                .block_on(
                    self.feeds
                        .fetch_snapshot_at_timestamp(timestamp_unix, reqs.as_deref()),
                )
                .map_err(map_env_error)?;
            PyEth15mObservation::from_snapshot(py, snapshot)
        }

        #[pyo3(signature = (start_timestamp, end_timestamp, step_blocks=1, pair_requests=None))]
        fn block_series_between(
            &self,
            py: Python<'_>,
            start_timestamp: i64,
            end_timestamp: i64,
            step_blocks: u64,
            pair_requests: Option<Vec<PyPairRequest>>,
        ) -> PyResult<Vec<PyEth15mObservation>> {
            if end_timestamp <= start_timestamp {
                return Ok(Vec::new());
            }
            let step = step_blocks.max(1);
            let reqs = convert_pair_requests(pair_requests)?;
            let start_block = self
                .rt
                .block_on(self.feeds.block_at_timestamp(start_timestamp))
                .map_err(map_env_error)?;
            let end_block = self
                .rt
                .block_on(self.feeds.block_at_timestamp(end_timestamp))
                .map_err(map_env_error)?;
            if end_block < start_block {
                return Ok(Vec::new());
            }
            let mut block = start_block;
            let mut out = Vec::new();
            loop {
                let snapshot = self
                    .rt
                    .block_on(self.feeds.fetch_snapshot_at_block(block, reqs.as_deref()))
                    .map_err(map_env_error)?;
                out.push(PyEth15mObservation::from_snapshot(py, snapshot)?);
                if block >= end_block {
                    break;
                }
                match block.checked_add(step) {
                    Some(next) if next <= end_block => block = next,
                    _ => break,
                }
            }
            Ok(out)
        }

        #[pyo3(signature = (start_timestamp, end_timestamp, interval_minutes=15))]
        fn polymarket_targets(
            &self,
            start_timestamp: i64,
            end_timestamp: i64,
            interval_minutes: u32,
        ) -> PyResult<Vec<PyPolymarketTarget>> {
            if interval_minutes == 0 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "interval_minutes must be positive",
                ));
            }
            if end_timestamp <= start_timestamp {
                return Ok(Vec::new());
            }
            let interval_secs = i64::from(interval_minutes) * 60;
            let mut cursor = align_timestamp(start_timestamp, interval_secs, true);
            let stop = align_timestamp(end_timestamp, interval_secs, false);
            if cursor >= stop {
                return Ok(Vec::new());
            }
            let mut out = Vec::new();
            let mut index = 0usize;
            while cursor < stop {
                let next = cursor + interval_secs;
                let start_block = self
                    .rt
                    .block_on(self.feeds.block_at_timestamp(cursor))
                    .map_err(map_env_error)?;
                let end_block = self
                    .rt
                    .block_on(self.feeds.block_at_timestamp(next))
                    .map_err(map_env_error)?;
                out.push(PyPolymarketTarget::new(
                    index,
                    cursor,
                    next,
                    start_block,
                    end_block,
                ));
                index += 1;
                cursor = next;
            }
            Ok(out)
        }
    }

    fn align_timestamp(timestamp: i64, interval_secs: i64, forward: bool) -> i64 {
        if interval_secs <= 0 {
            return timestamp;
        }
        let remainder = timestamp % interval_secs;
        if remainder == 0 {
            timestamp
        } else if forward {
            timestamp + (interval_secs - remainder)
        } else {
            timestamp - remainder
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
    #[derive(Clone)]
    pub struct PyPolymarketTarget;

    #[pyclass]
    pub struct PyWindowRecord;

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
    impl PyPolymarketTarget {
        #[new]
        fn new() -> PyResult<Self> {
            Err(disabled!("pyreth built without `prices` feature"))
        }
    }

    #[pymethods]
    impl PyWindowRecord {
        #[new]
        fn new() -> PyResult<Self> {
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
