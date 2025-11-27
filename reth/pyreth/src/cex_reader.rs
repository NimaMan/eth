use eth_prices::cex_readers::{CexPriceFeeder, CexPriceUpdate};
use pyo3::prelude::*;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, Mutex};

#[pyclass]
pub struct PyCexPriceUpdate {
    #[pyo3(get)]
    pub exchange: String,
    #[pyo3(get)]
    pub pair: String,
    #[pyo3(get)]
    pub bid: f64,
    #[pyo3(get)]
    pub ask: f64,
    #[pyo3(get)]
    pub received_at: f64,
    #[pyo3(get)]
    pub exchange_timestamp: Option<f64>,
}

impl From<CexPriceUpdate> for PyCexPriceUpdate {
    fn from(p: CexPriceUpdate) -> Self {
        let recv_ts = p
            .received_at
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
            
        let ex_ts = p.exchange_timestamp.map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        });

        Self {
            exchange: p.exchange,
            pair: p.pair,
            bid: p.bid,
            ask: p.ask,
            received_at: recv_ts,
            exchange_timestamp: ex_ts,
        }
    }
}

#[pyclass]
pub struct PyCexPriceFeeder {
    rt: Arc<Runtime>,
    receiver: Arc<Mutex<Option<mpsc::Receiver<CexPriceUpdate>>>>,
}

#[pymethods]
impl PyCexPriceFeeder {
    #[new]
    fn new(pairs: Vec<String>) -> PyResult<Self> {
        let rt = Arc::new(Runtime::new().unwrap());
        let receiver_arc = Arc::new(Mutex::new(None));
        
        let receiver_clone = receiver_arc.clone();
        let pairs_clone = pairs.clone();

        // We need to initialize and start the feeder on the async runtime
        rt.block_on(async move {
            let mut feeder = CexPriceFeeder::new()
                .with_binance()
                .with_coinbase()
                .with_kraken()
                .with_bybit()
                .with_okx()
                .with_gate()
                .with_bitfinex();

            // Start connections
            if let Err(e) = feeder.start(pairs_clone).await {
                eprintln!("Error starting CEX feeder: {}", e);
            }

            // Spawn aggregation task and get receiver
            let rx = feeder.spawn_aggregation();
            *receiver_clone.lock().await = Some(rx);
        });

        Ok(Self {
            rt,
            receiver: receiver_arc,
        })
    }

    fn get_update(&self, py: Python) -> PyResult<Option<PyCexPriceUpdate>> {
        let receiver = self.receiver.clone();
        let rt = self.rt.clone();
        
        let update = py.allow_threads(move || {
            rt.block_on(async {
                let mut rx_guard = receiver.lock().await;
                if let Some(rx) = rx_guard.as_mut() {
                    match rx.try_recv() {
                        Ok(u) => Ok(Some(u)),
                        Err(mpsc::error::TryRecvError::Empty) => Ok(None),
                        Err(mpsc::error::TryRecvError::Disconnected) => Err("Channel disconnected"),
                    }
                } else {
                    Ok(None)
                }
            })
        });

        match update {
            Ok(Some(u)) => Ok(Some(PyCexPriceUpdate::from(u))),
            Ok(None) => Ok(None),
            Err(_) => Err(pyo3::exceptions::PyRuntimeError::new_err("Channel disconnected")),
        }
    }
}
