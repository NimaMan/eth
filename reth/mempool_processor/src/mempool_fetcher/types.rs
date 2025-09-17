use std::time::Instant;

use ethers::types::U256;
use serde_json::Value;

use crate::function_detector::CreatorFunctionType;

/// Transaction received from the mempool via the IPC client.
///
/// The fetcher populates frequently used fields (addresses, calldata, gas
/// pricing) up front so the rest of the pipeline can avoid repeatedly parsing
/// JSON.
#[derive(Debug, Clone)]
pub struct MempoolTransaction {
    pub hash: String,
    pub data: Value,
    pub detection_ns: u64,
    pub detection_time: Instant,
    pub latency_ns: u64,
    pub from: Vec<u8>,
    pub to: Option<Vec<u8>>,
    pub input: Vec<u8>,
    pub value: U256,
    pub gas_price: Option<U256>,
    pub functions: Vec<String>,
    pub function_category: Option<CreatorFunctionType>,
}
