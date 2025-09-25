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
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Arc;

use alloy_primitives::Address;
use alloy_primitives::B256 as RB256;
use reth_chain_query::common_addresses::find_uniswap_v4_pools_for_pair;
use reth_chain_query::provider::BalanceDiff;
use reth_chain_query::tx_builders::amm_swap_route::AmmSwapRoute;
use reth_chain_query::{Account, BalanceChanges, CompleteBalances, Portfolio, RethQueryProvider};
use tokio::runtime::Runtime;

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
        let dict = PyDict::new(py);
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
        let dict = PyDict::new(py);
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
        let dict = PyDict::new(py);
        Ok(dict.into())
    }
}

/// Main ChainQuery wrapper for Python
#[pyclass(name = "ChainQuery")]
pub struct PyChainQuery {
    pub(super) runtime: Arc<Runtime>,
    pub(super) provider: Arc<RethQueryProvider>,
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

        let provider = RethQueryProvider::with_simulator(simulator)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self {
            runtime: Arc::new(runtime),
            provider: Arc::new(provider),
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

        let dict = PyDict::new(py);
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
        let fut = find_uniswap_v4_pools_for_pair(&sim, pm, a, b, blocks_back, max_results);
        let v = self
            .runtime
            .block_on(async move { fut.await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let mut out = Vec::with_capacity(v.len());
        for info in v {
            let d = PyDict::new(py);
            d.set_item("pool_id", format!("0x{:x}", info.pool_id))?;
            d.set_item("currency0", format!("0x{}", hex::encode(info.currency0)))?;
            d.set_item("currency1", format!("0x{}", hex::encode(info.currency1)))?;
            d.set_item("fee", info.fee)?;
            d.set_item("tick_spacing", info.tick_spacing)?;
            d.set_item("hooks", format!("0x{}", hex::encode(info.hooks)))?;
            d.set_item("block_number", info.block_number)?;
            out.push(d.into());
        }
        Ok(out)
    }

    /// Get ERC20 token decimals via on-chain query
    fn get_token_decimals(&self, token: &str, block: Option<u64>) -> PyResult<u8> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        self.runtime
            .block_on(async move { provider.get_token_decimals(token_addr, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get ERC20 token symbol via on-chain query
    fn get_token_symbol(&self, token: &str, block: Option<u64>) -> PyResult<String> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        self.runtime
            .block_on(async move { provider.get_token_symbol(token_addr, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get ERC20 token name via on-chain query
    fn get_token_name(&self, token: &str, block: Option<u64>) -> PyResult<String> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        self.runtime
            .block_on(async move { provider.get_token_name(token_addr, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get ERC20 total supply as string (wei) via on-chain query
    fn get_token_total_supply(&self, token: &str, block: Option<u64>) -> PyResult<String> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        let supply = self
            .runtime
            .block_on(async move { provider.get_token_total_supply(token_addr, block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(supply.to_string())
    }

    /// Get complete token metadata in a single call
    fn get_token_metadata(&self, token: &str) -> PyResult<super::tokens::PyTokenMetadata> {
        let token_addr = super::utils::parse_address(token)?;
        let provider = self.provider.clone();
        let meta = self
            .runtime
            .block_on(async move { provider.get_token_metadata(token_addr).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(super::tokens::PyTokenMetadata {
            address: format!("0x{}", hex::encode(meta.address)),
            name: meta.name,
            symbol: meta.symbol,
            decimals: meta.decimals,
            total_supply: meta.total_supply.to_string(),
        })
    }
}

// parse_address moved to utils.rs
