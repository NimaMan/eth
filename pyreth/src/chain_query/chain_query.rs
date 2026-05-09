/// Python bindings for reth_chain_query
///
/// Provides direct blockchain data access through Python, focusing on
/// address-related operations (balances, nonces, portfolio management).
/// Transaction operations are handled by tx_processor module.
///
/// Algorithm:
/// 1. Wrap RethQueryProvider for Python access
/// 2. Convert between Rust and Python types (Address <-> str, U256 <-> int/str)
/// 3. Expose single and batch operations for efficiency
/// 4. Track balance changes between blocks
/// 5. Share TxSimulator instance with other PyReth components
use chrono::SecondsFormat;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::tx_processor::processed_tx_bridge::processed_transaction_hashes_from_py_iterable;
use alloy_primitives::Address;
use alloy_primitives::B256 as RB256;
use reth_chain_query::dex::find_uniswap_v4_pools_for_pair;
use reth_chain_query::provider::{BalanceDiff, TransactionData as RustTransactionData};
use reth_chain_query::reth_index::RethIndexDB;
use reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute;
use reth_chain_query::BlockTimeConverter;
use reth_chain_query::{Account, BalanceChanges, CompleteBalances, Portfolio, RethQueryProvider};
use tokio::runtime::Runtime;

static RETH_INDEX_DB: Lazy<Mutex<Option<(PathBuf, Arc<RethIndexDB>)>>> =
    Lazy::new(|| Mutex::new(None));

pub(crate) fn shared_reth_index_db(index_dir: &Path) -> PyResult<Arc<RethIndexDB>> {
    let cache_key = index_dir
        .canonicalize()
        .unwrap_or_else(|_| index_dir.to_path_buf());
    let mut cached = RETH_INDEX_DB.lock();

    if let Some((cached_path, db)) = cached.as_ref() {
        if cached_path == &cache_key {
            return Ok(db.clone());
        }
    }

    let db = Arc::new(RethIndexDB::open_read_only(index_dir).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to open Reth address index at {}: {}",
            index_dir.display(),
            e
        ))
    })?);
    *cached = Some((cache_key, db.clone()));
    Ok(db)
}

pub(crate) fn clear_reth_index_db_cache() {
    *RETH_INDEX_DB.lock() = None;
}

/// Python wrapper for Account information
#[pyclass(name = "Account")]
#[derive(Clone)]
pub struct PyAccount {
    #[pyo3(get)]
    pub address: String,
    #[pyo3(get)]
    pub nonce: u64,
    #[pyo3(get)]
    pub balance: String,
    #[pyo3(get)]
    pub is_contract: bool,
}

impl PyAccount {
    fn from_account(address: Address, account: Account) -> Self {
        Self {
            address: format!("0x{}", hex::encode(address)),
            nonce: account.nonce,
            balance: account.balance.to_string(),
            is_contract: account.code_hash.is_some(),
        }
    }
}

/// Python wrapper for Portfolio information
#[pyclass(name = "Portfolio")]
#[derive(Clone)]
pub struct PyPortfolio {
    #[pyo3(get)]
    pub address: String,
    #[pyo3(get)]
    pub eth_balance: String,
    #[pyo3(get)]
    pub block_number: u64,
}

#[pymethods]
impl PyPortfolio {
    /// Get token balances as a dictionary
    fn token_balances(&self, py: Python) -> PyResult<PyObject> {
        // This will be populated by the actual Portfolio data
        let dict = PyDict::new_bound(py);
        Ok(dict.into())
    }
}

/// Python wrapper for balance change information
#[pyclass(name = "BalanceChange")]
#[derive(Clone)]
pub struct PyBalanceChange {
    #[pyo3(get)]
    pub before: String,
    #[pyo3(get)]
    pub after: String,
    #[pyo3(get)]
    pub difference: String,
    #[pyo3(get)]
    pub is_increase: bool,
}

/// Python wrapper for complete balance changes
#[pyclass(name = "BalanceChanges")]
#[derive(Clone)]
pub struct PyBalanceChanges {
    #[pyo3(get)]
    pub address: String,
    #[pyo3(get)]
    pub from_block: u64,
    #[pyo3(get)]
    pub to_block: u64,
    #[pyo3(get)]
    pub eth_change: PyBalanceChange,
}

#[pymethods]
impl PyBalanceChanges {
    /// Get token changes as a dictionary
    fn token_changes(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        Ok(dict.into())
    }
}

/// Python wrapper for complete balances at a block
#[pyclass(name = "CompleteBalances")]
#[derive(Clone)]
pub struct PyCompleteBalances {
    #[pyo3(get)]
    pub address: String,
    #[pyo3(get)]
    pub eth_balance: String,
    #[pyo3(get)]
    pub block_number: u64,
}

#[pymethods]
impl PyCompleteBalances {
    /// Get token balances as a dictionary
    fn token_balances(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        Ok(dict.into())
    }
}

#[pyclass(name = "TransactionData")]
#[derive(Clone)]
pub struct PyTransactionData {
    #[pyo3(get)]
    pub hash: String,
    #[pyo3(get)]
    pub block_number: u64,
    #[pyo3(get)]
    pub block_timestamp: u64,
    #[pyo3(get)]
    pub tx_index: u64,
    #[pyo3(get)]
    pub tx_number: u64,
    #[pyo3(get)]
    pub from_address: String,
    #[pyo3(get)]
    pub to_address: Option<String>,
    #[pyo3(get)]
    pub value: String,
    #[pyo3(get)]
    pub gas_price: String,
    #[pyo3(get)]
    pub gas_limit: u64,
    #[pyo3(get)]
    pub nonce: u64,
    #[pyo3(get)]
    pub transaction_type: u8,
}

impl From<RustTransactionData> for PyTransactionData {
    fn from(data: RustTransactionData) -> Self {
        Self {
            hash: format!("0x{}", hex::encode(data.hash.as_slice())),
            block_number: data.block_number,
            block_timestamp: data.block_timestamp,
            tx_index: data.tx_index,
            tx_number: data.tx_number,
            from_address: format!("0x{}", hex::encode(data.from.as_slice())),
            to_address: data
                .to
                .map(|addr| format!("0x{}", hex::encode(addr.as_slice()))),
            value: data.value.to_string(),
            gas_price: data.gas_price.to_string(),
            gas_limit: data.gas_limit,
            nonce: data.nonce,
            transaction_type: data.transaction_type,
        }
    }
}

/// Main ChainQuery wrapper for Python
#[pyclass(name = "ChainQuery")]
pub struct PyChainQuery {
    pub(super) runtime: Arc<Runtime>,
    pub(super) provider: Arc<RethQueryProvider>,
    time_converter: Arc<BlockTimeConverter>,
    // Store actual data for Python access
    portfolios: parking_lot::RwLock<HashMap<String, Portfolio>>,
    balance_changes: parking_lot::RwLock<HashMap<String, BalanceChanges>>,
    complete_balances: parking_lot::RwLock<HashMap<String, CompleteBalances>>,
}

impl PyChainQuery {
    /// Create from shared TxSimulator
    pub fn from_simulator(simulator: Arc<tx_simulator::TxSimulator>) -> PyResult<Self> {
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let datadir = std::env::var("PYRETH_DATADIR")
            .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
        let index_dir = std::env::var("PYRETH_ADDRESS_INDEX_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Path::new(&datadir).join("reth_index"));

        let index_db = shared_reth_index_db(&index_dir)?;

        let time_converter = Arc::new(BlockTimeConverter::new(simulator.clone()));

        let provider = RethQueryProvider::with_simulator(simulator)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?
            .with_reth_index_db(index_db);

        Ok(Self {
            runtime: Arc::new(runtime),
            provider: Arc::new(provider),
            time_converter,
            portfolios: parking_lot::RwLock::new(HashMap::new()),
            balance_changes: parking_lot::RwLock::new(HashMap::new()),
            complete_balances: parking_lot::RwLock::new(HashMap::new()),
        })
    }
}

#[pymethods]
impl PyChainQuery {
    /// Get the latest block number
    fn get_latest_block(&self) -> PyResult<u64> {
        self.provider
            .get_latest_block()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Convert a block number to an ISO8601 timestamp (UTC)
    fn block_to_timestamp(&self, block_number: u64) -> PyResult<String> {
        let timestamp =
            super::utils::block_to_timestamp(&self.runtime, &self.time_converter, block_number)?;

        Ok(timestamp.to_rfc3339_opts(SecondsFormat::Secs, true))
    }

    /// Convert an ISO8601 timestamp (UTC) to the nearest block at or before that time
    fn timestamp_to_block(&self, timestamp: &str) -> PyResult<u64> {
        let parsed = super::utils::parse_iso_timestamp(timestamp)?;
        super::utils::timestamp_to_block_floor(&self.runtime, &self.time_converter, parsed)
    }

    /// Get ETH balance for an address
    ///
    /// Args:
    ///     address: Ethereum address (with or without 0x prefix)
    ///     block: Optional block number (defaults to latest)
    ///
    /// Returns:
    ///     ETH balance as string (in wei)
    fn get_eth_balance(&self, address: &str, block: Option<u64>) -> PyResult<String> {
        let addr = super::utils::parse_address(address)?;

        let provider = self.provider.clone();
        let balance = self
            .runtime
            .block_on(async move { provider.get_eth_balance(addr, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(balance.to_string())
    }

    /// Get token balance for an address
    ///
    /// Args:
    ///     token: Token contract address
    ///     holder: Holder address
    ///     block: Optional block number
    ///
    /// Returns:
    ///     Token balance as string (in smallest unit)
    fn get_token_balance(&self, token: &str, holder: &str, block: Option<u64>) -> PyResult<String> {
        let token_addr = super::utils::parse_address(token)?;
        let holder_addr = super::utils::parse_address(holder)?;

        let provider = self.provider.clone();
        let balance = self
            .runtime
            .block_on(async move {
                provider
                    .get_token_balance(token_addr, holder_addr, block)
                    .await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(balance.to_string())
    }

    /// Get all indexed processed blocks that involve the given address.
    fn address_participation_blocks(&self, address: &str) -> PyResult<Vec<u64>> {
        let addr = super::utils::parse_address(address)?;
        self.provider
            .participation_blocks_for_address(addr)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Fetch complete transaction metadata by global sequential number (Txumber).
    fn transaction_by_number(&self, tx_number: u64) -> PyResult<PyTransactionData> {
        let provider = self.provider.clone();
        let tx = self
            .runtime
            .block_on(async move { provider.get_transaction_by_number(tx_number).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(PyTransactionData::from(tx))
    }

    /// Get mempool arrival timestamp (milliseconds since epoch) if recorded
    fn get_tx_arrival_ms(&self, tx_hash: &str) -> PyResult<Option<u64>> {
        let hash = super::utils::parse_hash(tx_hash)?;
        self.provider
            .get_tx_arrival_ms(hash)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Fetch transaction metadata by hash.
    fn transaction_by_hash(&self, tx_hash: &str) -> PyResult<PyTransactionData> {
        let hash = super::utils::parse_hash(tx_hash)?;
        let provider = self.provider.clone();
        let tx = self
            .runtime
            .block_on(async move { provider.get_transaction_by_hash(hash).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(PyTransactionData::from(tx))
    }

    /// Return `(first_tx_number, tx_count)` for the specified block.
    fn block_tx_indices(&self, block_number: u64) -> PyResult<(u64, u64)> {
        let indices = self
            .provider
            .get_block_tx_indices(block_number)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok((indices.first_tx_num, indices.tx_count))
    }

    /// Whether address index entries exist for the given block number.
    fn block_has_indices(&self, block_number: u64) -> PyResult<bool> {
        Ok(self.provider.get_block_tx_indices(block_number).is_ok())
    }

    /// Get account information (nonce, balance, is_contract)
    ///
    /// Args:
    ///     address: Ethereum address
    ///     block: Optional block number
    ///
    /// Returns:
    ///     Account object with nonce, balance, and contract status
    fn get_account(&self, address: &str, block: Option<u64>) -> PyResult<PyAccount> {
        let addr = super::utils::parse_address(address)?;

        let provider = self.provider.clone();
        let account = self
            .runtime
            .block_on(async move { provider.get_account(addr, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyAccount::from_account(addr, account))
    }

    /// Get transaction count (nonce) for an address
    ///
    /// Args:
    ///     address: Ethereum address
    ///     block: Optional block number
    ///
    /// Returns:
    ///     Transaction count (nonce) as integer
    fn get_nonce(&self, address: &str, block: Option<u64>) -> PyResult<u64> {
        let addr = super::utils::parse_address(address)?;

        let provider = self.provider.clone();
        let nonce = self
            .runtime
            .block_on(async move { provider.get_transaction_count_at_block(addr, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(nonce)
    }

    /// Check if an address is a contract
    ///
    /// Args:
    ///     address: Ethereum address
    ///     block: Optional block number
    ///
    /// Returns:
    ///     True if address has code, False otherwise
    fn is_contract(&self, address: &str, block: Option<u64>) -> PyResult<bool> {
        let addr = super::utils::parse_address(address)?;

        let provider = self.provider.clone();
        let is_contract = self
            .runtime
            .block_on(async move { provider.has_code(addr, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(is_contract)
    }

    /// Get complete portfolio for an address
    ///
    /// Args:
    ///     address: Ethereum address
    ///     tokens: List of token addresses to check
    ///     block: Optional block number
    ///
    /// Returns:
    ///     Portfolio object with ETH and token balances
    fn get_portfolio(
        &self,
        address: &str,
        tokens: Vec<String>,
        block: Option<u64>,
    ) -> PyResult<PyPortfolio> {
        let addr = super::utils::parse_address(address)?;
        let token_addrs = tokens
            .iter()
            .map(|t| super::utils::parse_address(t))
            .collect::<PyResult<Vec<_>>>()?;

        let provider = self.provider.clone();
        let portfolio = self
            .runtime
            .block_on(async move { provider.get_portfolio(addr, token_addrs, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Store the portfolio for later access
        let key = format!("0x{}", hex::encode(addr));
        self.portfolios
            .write()
            .insert(key.clone(), portfolio.clone());

        Ok(PyPortfolio {
            address: key,
            eth_balance: portfolio.eth_balance.to_string(),
            block_number: portfolio.block_number,
        })
    }

    /// Get ETH balances for multiple addresses
    ///
    /// Args:
    ///     addresses: List of Ethereum addresses
    ///     block: Optional block number
    ///
    /// Returns:
    ///     List of balance strings in the same order
    fn batch_get_eth_balances(
        &self,
        addresses: Vec<String>,
        block: Option<u64>,
    ) -> PyResult<Vec<String>> {
        let addrs = addresses
            .iter()
            .map(|a| super::utils::parse_address(a))
            .collect::<PyResult<Vec<_>>>()?;

        let provider = self.provider.clone();
        let balances = self
            .runtime
            .block_on(async move {
                provider
                    .get_eth_balances_for_multiple_addresses(addrs, block)
                    .await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(balances.into_iter().map(|b| b.to_string()).collect())
    }

    /// Get token balances for multiple address/token pairs
    ///
    /// Args:
    ///     requests: List of (token_address, holder_address) tuples
    ///     block: Optional block number
    ///
    /// Returns:
    ///     List of balance strings in the same order
    fn batch_get_token_balances(
        &self,
        requests: Vec<(String, String)>,
        block: Option<u64>,
    ) -> PyResult<Vec<String>> {
        let pairs = requests
            .iter()
            .map(|(token, holder)| {
                Ok((
                    super::utils::parse_address(token)?,
                    super::utils::parse_address(holder)?,
                ))
            })
            .collect::<PyResult<Vec<_>>>()?;

        let provider = self.provider.clone();
        let balances = self
            .runtime
            .block_on(async move {
                provider
                    .batch_get_balances_for_token_holder_pairs(pairs, block)
                    .await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(balances.into_iter().map(|b| b.to_string()).collect())
    }

    /// Check which addresses are contracts
    ///
    /// Args:
    ///     addresses: List of addresses to check
    ///     block: Optional block number
    ///
    /// Returns:
    ///     Dictionary of address -> is_contract
    fn batch_is_contract(
        &self,
        py: Python,
        addresses: Vec<String>,
        block: Option<u64>,
    ) -> PyResult<PyObject> {
        let addrs = addresses
            .iter()
            .map(|a| super::utils::parse_address(a))
            .collect::<PyResult<Vec<_>>>()?;

        let provider = self.provider.clone();
        let results = self
            .runtime
            .block_on(async move { provider.batch_check_contracts(addrs, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let dict = PyDict::new_bound(py);
        for (addr, is_contract) in results {
            let addr_str = format!("0x{}", hex::encode(addr));
            dict.set_item(addr_str, is_contract)?;
        }

        Ok(dict.into())
    }

    /// Get complete balances (ETH + tokens) at a specific block
    ///
    /// Args:
    ///     address: Ethereum address
    ///     tokens: List of token addresses
    ///     block: Optional block number
    ///
    /// Returns:
    ///     CompleteBalances object
    fn get_complete_balances(
        &self,
        address: &str,
        tokens: Vec<String>,
        block: Option<u64>,
    ) -> PyResult<PyCompleteBalances> {
        let addr = super::utils::parse_address(address)?;
        let token_addrs = tokens
            .iter()
            .map(|t| super::utils::parse_address(t))
            .collect::<PyResult<Vec<_>>>()?;

        let provider = self.provider.clone();
        let balances = self
            .runtime
            .block_on(async move {
                provider
                    .get_complete_balances(addr, token_addrs, block)
                    .await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Store for later access
        let key = format!("0x{}", hex::encode(addr));
        self.complete_balances
            .write()
            .insert(key.clone(), balances.clone());

        Ok(PyCompleteBalances {
            address: key,
            eth_balance: balances.eth_balance.to_string(),
            block_number: balances.block_number,
        })
    }

    /// Get balance changes between two blocks
    ///
    /// Args:
    ///     address: Ethereum address
    ///     tokens: List of token addresses to track
    ///     from_block: Starting block number
    ///     to_block: Ending block number
    ///
    /// Returns:
    ///     BalanceChanges object showing differences
    fn get_balance_changes(
        &self,
        address: &str,
        tokens: Vec<String>,
        from_block: u64,
        to_block: u64,
    ) -> PyResult<PyBalanceChanges> {
        let addr = super::utils::parse_address(address)?;
        let token_addrs = tokens
            .iter()
            .map(|t| super::utils::parse_address(t))
            .collect::<PyResult<Vec<_>>>()?;

        let provider = self.provider.clone();
        let changes = self
            .runtime
            .block_on(async move {
                provider
                    .calculate_eth_and_token_balance_diff_between_blocks(
                        addr,
                        token_addrs,
                        from_block,
                        to_block,
                    )
                    .await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Convert ETH change
        let eth_change = PyBalanceChange {
            before: changes.eth_change.before.to_string(),
            after: changes.eth_change.after.to_string(),
            difference: match &changes.eth_change.difference {
                BalanceDiff::Increase(v) => v.to_string(),
                BalanceDiff::Decrease(v) => format!("-{}", v),
            },
            is_increase: matches!(changes.eth_change.difference, BalanceDiff::Increase(_)),
        };

        // Store for later access
        let key = format!("0x{}", hex::encode(addr));
        self.balance_changes.write().insert(key.clone(), changes);

        Ok(PyBalanceChanges {
            address: key,
            from_block,
            to_block,
            eth_change,
        })
    }

    /// Get Uniswap V2 pool liquidity (reserves) at a block
    #[pyo3(signature = (pool, block=None))]
    fn get_uniswap_v2_liquidity(
        &self,
        pool: &str,
        block: Option<u64>,
    ) -> PyResult<super::amm::PyPoolLiquidityInfo> {
        let pool_addr = super::utils::parse_address(pool)?;
        let route = AmmSwapRoute::UniswapV2 { pool: pool_addr };
        super::amm::get_pool_liquidity(self.runtime.clone(), self.provider.clone(), route, block)
    }

    /// Get Sushiswap V2 pool liquidity (reserves) at a block
    #[pyo3(signature = (pool, block=None))]
    fn get_sushiswap_v2_liquidity(
        &self,
        pool: &str,
        block: Option<u64>,
    ) -> PyResult<super::amm::PyPoolLiquidityInfo> {
        let pool_addr = super::utils::parse_address(pool)?;
        let route = AmmSwapRoute::SushiswapV2 { pool: pool_addr };
        super::amm::get_pool_liquidity(self.runtime.clone(), self.provider.clone(), route, block)
    }

    /// Get Uniswap V3 pool liquidity/tick at a block
    #[pyo3(signature = (pool, fee_tier, block=None))]
    fn get_uniswap_v3_liquidity(
        &self,
        pool: &str,
        fee_tier: u32,
        block: Option<u64>,
    ) -> PyResult<super::amm::PyPoolLiquidityInfo> {
        let pool_addr = super::utils::parse_address(pool)?;
        let route = AmmSwapRoute::UniswapV3 {
            pool: pool_addr,
            fee_tier,
        };
        super::amm::get_pool_liquidity(self.runtime.clone(), self.provider.clone(), route, block)
    }

    /// Get Uniswap V4 pool liquidity/tick via PoolManager and PoolId
    #[pyo3(signature = (pool_manager, pool_id_hex, block=None))]
    fn get_uniswap_v4_liquidity(
        &self,
        pool_manager: &str,
        pool_id_hex: &str,
        block: Option<u64>,
    ) -> PyResult<super::amm::PyPoolLiquidityInfo> {
        let pm = super::utils::parse_address(pool_manager)?;
        let pid_clean = pool_id_hex.trim_start_matches("0x");
        let mut arr = [0u8; 32];
        hex::decode_to_slice(pid_clean, &mut arr).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid pool id: {}", e))
        })?;
        let pid = RB256::from(arr);
        let route = AmmSwapRoute::UniswapV4 {
            pool_manager: pm,
            pool_id: pid,
        };
        super::amm::get_pool_liquidity(self.runtime.clone(), self.provider.clone(), route, block)
    }

    /// Find a recent Uniswap V4 pool id by scanning Initialize events on PoolManager.
    /// Returns hex string pool_id if found (without 0x prefix), else None.
    fn find_recent_uniswap_v4_pool_id(
        &self,
        pool_manager: &str,
        blocks_back: u64,
    ) -> PyResult<Option<String>> {
        let pm = super::utils::parse_address(pool_manager)?;
        let provider = self.provider.clone();
        let res = self
            .runtime
            .block_on(async move { provider.uni_v4_find_recent_pool_id(pm, blocks_back).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(res.map(|(pid, _block)| format!("{:x}", pid)))
    }

    /// Find recent Uniswap V4 pool ids for a token pair via PoolManager Initialize events.
    /// Returns up to `max_results` dict entries with pool_id, fee, tick_spacing, hooks, block.
    fn find_uniswap_v4_pools_for_pair(
        &self,
        py: Python,
        pool_manager: &str,
        token_a: &str,
        token_b: &str,
        blocks_back: u64,
        max_results: usize,
    ) -> PyResult<Vec<PyObject>> {
        let pm = super::utils::parse_address(pool_manager)?;
        let a = super::utils::parse_address(token_a)?;
        let b = super::utils::parse_address(token_b)?;

        // Access simulator directly for the discovery helper
        let sim = self.provider.simulator().clone();
        let block_hint = if blocks_back == 0 {
            None
        } else {
            Some(blocks_back)
        };
        let fut = find_uniswap_v4_pools_for_pair(&sim, pm, a, b, block_hint);
        let v = self
            .runtime
            .block_on(async move { fut.await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let mut out = Vec::with_capacity(v.len().min(max_results));
        for info in v.into_iter().take(max_results) {
            let d = PyDict::new_bound(py);
            d.set_item("pool_id", format!("0x{:x}", info.pool_id))?;
            d.set_item(
                "pool_address",
                format!("0x{}", hex::encode(info.pool_address)),
            )?;
            d.set_item("fee", info.fee)?;
            d.set_item("tick_spacing", info.tick_spacing)?;
            d.set_item("hooks", format!("0x{}", hex::encode(info.hooks)))?;
            out.push(d.into());
        }
        Ok(out)
    }

    /// Get ERC20 token decimals via on-chain query
    #[pyo3(signature = (token, block_number=None))]
    fn get_token_decimals(&self, token: &str, block_number: Option<u64>) -> PyResult<u8> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        self.runtime
            .block_on(async move { provider.get_token_decimals(token_addr, block_number).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get ERC20 token symbol via on-chain query
    #[pyo3(signature = (token, block_number=None))]
    fn get_token_symbol(&self, token: &str, block_number: Option<u64>) -> PyResult<String> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        self.runtime
            .block_on(async move { provider.get_token_symbol(token_addr, block_number).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get ERC20 token name via on-chain query
    #[pyo3(signature = (token, block_number=None))]
    fn get_token_name(&self, token: &str, block_number: Option<u64>) -> PyResult<String> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        self.runtime
            .block_on(async move { provider.get_token_name(token_addr, block_number).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get ERC20 total supply as string (wei) via on-chain query
    #[pyo3(signature = (token, block_number=None))]
    fn get_token_total_supply(&self, token: &str, block_number: Option<u64>) -> PyResult<String> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        let supply = self
            .runtime
            .block_on(async move {
                provider
                    .get_token_total_supply(token_addr, block_number)
                    .await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(supply.to_string())
    }

    /// Get complete token metadata in a single call
    #[pyo3(signature = (token, block_number=None, pending_transactions=None, gas_block_number=None))]
    fn get_token_metadata(
        &self,
        token: &str,
        block_number: Option<u64>,
        pending_transactions: Option<&Bound<'_, PyAny>>,
        gas_block_number: Option<u64>,
    ) -> PyResult<Option<super::tokens::PyTokenMetadata>> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        let metadata_block = block_number.or(gas_block_number);
        let pending_tx_hashes: Vec<RB256> = if let Some(iterable) = pending_transactions {
            processed_transaction_hashes_from_py_iterable(iterable)?
        } else {
            Vec::new()
        };
        let meta = self
            .runtime
            .block_on(async move {
                let pending_hashes = if pending_tx_hashes.is_empty() {
                    None
                } else {
                    Some(pending_tx_hashes)
                };
                provider
                    .get_token_metadata(token_addr, metadata_block, pending_hashes)
                    .await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(meta.map(|inner| super::tokens::PyTokenMetadata {
            address: format!("0x{}", hex::encode(inner.address)),
            name: inner.name,
            symbol: inner.symbol,
            decimals: inner.decimals,
            total_supply: inner.total_supply.to_string(),
        }))
    }

    /// Estimate gas price based on recent block activity.
    ///
    /// Args:
    ///     percentile: Priority fee percentile (0.0 to 1.0). Default 0.5 (median).
    ///     block_number: Optional block to check. Default latest.
    ///
    /// Returns:
    ///     Estimated gas price (base_fee + priority_fee) in wei as string.
    #[pyo3(signature = (percentile=None, block_number=None))]
    fn estimate_gas_price(
        &self,
        percentile: Option<f64>,
        block_number: Option<u64>,
    ) -> PyResult<String> {
        let block = block_number.unwrap_or(self.get_latest_block()?);
        let p = percentile.unwrap_or(0.5).clamp(0.0, 1.0);

        let provider = self.provider.clone();
        let (base_fee, priority_fees) = self
            .runtime
            .block_on(async move {
                let block_txs = provider.get_block_transactions(block).await?;
                let base_fee = block_txs.base_fee_per_gas.unwrap_or_default() as u128;
                let mut priority_fees = block_txs
                    .transactions
                    .iter()
                    .map(|tx| {
                        let gas_price: u128 =
                            tx.tx_metadata.gas_price.try_into().unwrap_or(u128::MAX);
                        gas_price.saturating_sub(base_fee)
                    })
                    .collect::<Vec<_>>();
                priority_fees.sort_unstable();
                Ok::<_, eyre::Error>((base_fee, priority_fees))
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let priority_fee = if priority_fees.is_empty() {
            0
        } else {
            let idx = ((priority_fees.len() as f64 * p) as usize).min(priority_fees.len() - 1);
            priority_fees[idx]
        };

        let total_fee = base_fee + priority_fee;
        Ok(total_fee.to_string())
    }

    /// Estimate total transaction cost.
    ///
    /// Args:
    ///     gas_limit: Gas limit for the transaction.
    ///     priority: "low" (10%), "normal" (50%), "high" (90%), "urgent" (99%). Default "normal".
    ///     block_number: Optional block number. Default latest.
    ///
    /// Returns:
    ///     Estimated cost in wei as string.
    #[pyo3(signature = (gas_limit, priority="normal", block_number=None))]
    fn estimate_tx_cost(
        &self,
        gas_limit: u64,
        priority: &str,
        block_number: Option<u64>,
    ) -> PyResult<String> {
        let percentile = match priority {
            "low" => 0.10,
            "normal" => 0.50,
            "high" => 0.90,
            "urgent" => 0.99,
            _ => 0.50,
        };

        let price_str = self.estimate_gas_price(Some(percentile), block_number)?;
        let price: u128 = price_str.parse().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid gas price calculated")
        })?;

        let total_cost = price * gas_limit as u128;
        Ok(total_cost.to_string())
    }
}

// parse_address moved to utils.rs
