use alloy_primitives::Address;
use pyo3::{exceptions::PyValueError, prelude::*};
use reth_chain_query::common_addresses::{
    address_book::{get_address_by_name as rc_get_address_by_name, ADDRESSES_BY_NAME},
    cex::CEX_ADDRESSES,
    denom_tokens::{DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS, SYMBOL_TO_ADDRESS},
    dex_token_denom_pairs::{
        sushiswap_tokens, uniswap_v2_tokens, uniswap_v3_tokens, SushiSwapTokenInfo,
        UniswapV2TokenInfo, UniswapV3TokenInfo,
    },
    etf::ETF_ADDRESSES,
    pool_addresses::{
        get_pool_protocol as rc_get_pool_protocol, is_known_factory as rc_is_known_factory,
        is_v4_pool_manager as rc_is_v4_pool_manager, POOL_FACTORIES, ROUTERS,
    },
    stablecoins::STABLECOINS,
    validators::FEE_RECIPIENT_LIST,
};
use reth_chain_query::to_checksum_address;
use std::collections::HashMap;
use std::str::FromStr;

fn format_address(addr: Address) -> String {
    to_checksum_address(&addr)
}

fn parse_hex_address(input: &str) -> PyResult<Address> {
    Address::from_str(input).map_err(|err| {
        PyValueError::new_err(format!("Invalid Ethereum address '{}': {}", input, err))
    })
}

#[pyfunction]
pub fn address_by_name(name: &str) -> Option<String> {
    rc_get_address_by_name(name).map(format_address)
}

#[pyfunction]
pub fn address_book() -> HashMap<String, String> {
    ADDRESSES_BY_NAME
        .iter()
        .map(|(label, addr)| (label.to_string(), format_address(*addr)))
        .collect()
}

#[pyfunction]
pub fn pool_factories() -> HashMap<String, String> {
    POOL_FACTORIES
        .iter()
        .map(|(label, addr)| (label.to_string(), format_address(*addr)))
        .collect()
}

#[pyfunction]
pub fn routers() -> HashMap<String, String> {
    ROUTERS
        .iter()
        .map(|(label, addr)| (label.to_string(), format_address(*addr)))
        .collect()
}

#[pyfunction]
pub fn get_pool_protocol(address: &str) -> PyResult<Option<String>> {
    let addr = parse_hex_address(address)?;
    Ok(rc_get_pool_protocol(addr).map(|s| s.to_string()))
}

#[pyfunction]
pub fn is_known_factory(address: &str) -> PyResult<bool> {
    let addr = parse_hex_address(address)?;
    Ok(rc_is_known_factory(addr))
}

#[pyfunction]
pub fn is_uniswap_v4_pool_manager(address: &str) -> PyResult<bool> {
    let addr = parse_hex_address(address)?;
    Ok(rc_is_v4_pool_manager(addr))
}

#[pyfunction]
pub fn denom_symbol_map() -> HashMap<String, String> {
    SYMBOL_TO_ADDRESS
        .iter()
        .map(|(symbol, addr)| ((*symbol).to_string(), format_address(*addr)))
        .collect()
}

#[pyfunction]
pub fn denom_address_map() -> HashMap<String, String> {
    DENOM_ADDRESSES
        .iter()
        .map(|(addr, symbol)| (format_address(*addr), (*symbol).to_string()))
        .collect()
}

#[pyfunction]
pub fn token_decimals() -> HashMap<String, u8> {
    ERC20_TOKEN_DECIMALS
        .iter()
        .map(|(symbol, decimals)| ((*symbol).to_string(), *decimals))
        .collect()
}

#[pyfunction]
pub fn cex_address_map() -> HashMap<String, String> {
    CEX_ADDRESSES
        .iter()
        .map(|entry| (entry.name.to_string(), format_address(entry.address)))
        .collect()
}

#[pyfunction]
pub fn etf_address_map() -> HashMap<String, String> {
    ETF_ADDRESSES
        .iter()
        .map(|entry| (entry.name.to_string(), format_address(entry.address)))
        .collect()
}

#[pyclass(name = "FeeRecipient")]
pub struct PyFeeRecipient {
    address: String,
    name: String,
}

impl From<&'static reth_chain_query::common_addresses::validators::FeeRecipient>
    for PyFeeRecipient
{
    fn from(entry: &'static reth_chain_query::common_addresses::validators::FeeRecipient) -> Self {
        Self {
            address: format_address(entry.address),
            name: entry.name.to_string(),
        }
    }
}

#[pymethods]
impl PyFeeRecipient {
    #[getter]
    pub fn address(&self) -> &str {
        &self.address
    }

    #[getter]
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[pyfunction]
pub fn fee_recipients() -> HashMap<String, String> {
    FEE_RECIPIENT_LIST
        .iter()
        .map(|entry| (format_address(entry.address), entry.name.to_string()))
        .collect()
}

#[pyfunction]
pub fn fee_recipient_list() -> Vec<PyFeeRecipient> {
    FEE_RECIPIENT_LIST
        .iter()
        .map(PyFeeRecipient::from)
        .collect()
}

#[pyclass(name = "StablecoinInfo")]
pub struct PyStablecoinInfo {
    symbol: String,
    address: String,
    decimals: u8,
    unit: String,
}

impl From<&'static reth_chain_query::common_addresses::stablecoins::StablecoinInfo>
    for PyStablecoinInfo
{
    fn from(
        info: &'static reth_chain_query::common_addresses::stablecoins::StablecoinInfo,
    ) -> Self {
        Self {
            symbol: info.symbol.to_string(),
            address: format_address(info.address),
            decimals: info.decimals,
            unit: info.unit.to_string(),
        }
    }
}

#[pymethods]
impl PyStablecoinInfo {
    #[getter]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    #[getter]
    pub fn address(&self) -> &str {
        &self.address
    }

    #[getter]
    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    #[getter]
    pub fn unit(&self) -> &str {
        &self.unit
    }
}

#[pyfunction]
pub fn stablecoin_infos() -> Vec<PyStablecoinInfo> {
    STABLECOINS.iter().map(PyStablecoinInfo::from).collect()
}

#[pyclass(name = "UniswapV2PairInfo")]
pub struct PyUniswapV2PairInfo {
    symbol: String,
    token_address: String,
    decimals: u8,
    denom_symbol: String,
    denom_address: String,
    denom_decimals: u8,
}

impl From<&'static UniswapV2TokenInfo> for PyUniswapV2PairInfo {
    fn from(info: &'static UniswapV2TokenInfo) -> Self {
        Self {
            symbol: info.symbol.to_string(),
            token_address: format_address(info.token_address),
            decimals: info.decimals,
            denom_symbol: info.denom_symbol.to_string(),
            denom_address: format_address(info.denom_address),
            denom_decimals: info.denom_decimals,
        }
    }
}

#[pymethods]
impl PyUniswapV2PairInfo {
    #[getter]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    #[getter]
    pub fn token_address(&self) -> &str {
        &self.token_address
    }

    #[getter]
    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    #[getter]
    pub fn denom_symbol(&self) -> &str {
        &self.denom_symbol
    }

    #[getter]
    pub fn denom_address(&self) -> &str {
        &self.denom_address
    }

    #[getter]
    pub fn denom_decimals(&self) -> u8 {
        self.denom_decimals
    }
}

#[pyclass(name = "UniswapV3PairInfo")]
pub struct PyUniswapV3PairInfo {
    symbol: String,
    token_address: String,
    decimals: u8,
    denom_symbol: String,
    denom_address: String,
    denom_decimals: u8,
    fee_tier: u32,
}

impl From<&'static UniswapV3TokenInfo> for PyUniswapV3PairInfo {
    fn from(info: &'static UniswapV3TokenInfo) -> Self {
        Self {
            symbol: info.symbol.to_string(),
            token_address: format_address(info.token_address),
            decimals: info.decimals,
            denom_symbol: info.denom_symbol.to_string(),
            denom_address: format_address(info.denom_address),
            denom_decimals: info.denom_decimals,
            fee_tier: info.fee_tier,
        }
    }
}

#[pymethods]
impl PyUniswapV3PairInfo {
    #[getter]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    #[getter]
    pub fn token_address(&self) -> &str {
        &self.token_address
    }

    #[getter]
    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    #[getter]
    pub fn denom_symbol(&self) -> &str {
        &self.denom_symbol
    }

    #[getter]
    pub fn denom_address(&self) -> &str {
        &self.denom_address
    }

    #[getter]
    pub fn denom_decimals(&self) -> u8 {
        self.denom_decimals
    }

    #[getter]
    pub fn fee_tier(&self) -> u32 {
        self.fee_tier
    }
}

#[pyclass(name = "SushiSwapPairInfo")]
pub struct PySushiSwapPairInfo {
    symbol: String,
    token_address: String,
    decimals: u8,
    denom_symbol: String,
    denom_address: String,
    denom_decimals: u8,
}

impl From<&'static SushiSwapTokenInfo> for PySushiSwapPairInfo {
    fn from(info: &'static SushiSwapTokenInfo) -> Self {
        Self {
            symbol: info.symbol.to_string(),
            token_address: format_address(info.token_address),
            decimals: info.decimals,
            denom_symbol: info.denom_symbol.to_string(),
            denom_address: format_address(info.denom_address),
            denom_decimals: info.denom_decimals,
        }
    }
}

#[pymethods]
impl PySushiSwapPairInfo {
    #[getter]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    #[getter]
    pub fn token_address(&self) -> &str {
        &self.token_address
    }

    #[getter]
    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    #[getter]
    pub fn denom_symbol(&self) -> &str {
        &self.denom_symbol
    }

    #[getter]
    pub fn denom_address(&self) -> &str {
        &self.denom_address
    }

    #[getter]
    pub fn denom_decimals(&self) -> u8 {
        self.denom_decimals
    }
}

#[pyfunction]
pub fn uniswap_v2_pairs() -> Vec<PyUniswapV2PairInfo> {
    uniswap_v2_tokens()
        .iter()
        .map(PyUniswapV2PairInfo::from)
        .collect()
}

#[pyfunction]
pub fn uniswap_v3_pairs() -> Vec<PyUniswapV3PairInfo> {
    uniswap_v3_tokens()
        .iter()
        .map(PyUniswapV3PairInfo::from)
        .collect()
}

#[pyfunction]
pub fn sushiswap_pairs() -> Vec<PySushiSwapPairInfo> {
    sushiswap_tokens()
        .iter()
        .map(PySushiSwapPairInfo::from)
        .collect()
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyUniswapV2PairInfo>()?;
    module.add_class::<PyUniswapV3PairInfo>()?;
    module.add_class::<PySushiSwapPairInfo>()?;
    module.add_class::<PyFeeRecipient>()?;
    module.add_class::<PyStablecoinInfo>()?;
    module.add_function(wrap_pyfunction!(address_by_name, module)?)?;
    module.add_function(wrap_pyfunction!(address_book, module)?)?;
    module.add_function(wrap_pyfunction!(pool_factories, module)?)?;
    module.add_function(wrap_pyfunction!(routers, module)?)?;
    module.add_function(wrap_pyfunction!(get_pool_protocol, module)?)?;
    module.add_function(wrap_pyfunction!(is_known_factory, module)?)?;
    module.add_function(wrap_pyfunction!(is_uniswap_v4_pool_manager, module)?)?;
    module.add_function(wrap_pyfunction!(denom_symbol_map, module)?)?;
    module.add_function(wrap_pyfunction!(denom_address_map, module)?)?;
    module.add_function(wrap_pyfunction!(token_decimals, module)?)?;
    module.add_function(wrap_pyfunction!(cex_address_map, module)?)?;
    module.add_function(wrap_pyfunction!(etf_address_map, module)?)?;
    module.add_function(wrap_pyfunction!(fee_recipients, module)?)?;
    module.add_function(wrap_pyfunction!(fee_recipient_list, module)?)?;
    module.add_function(wrap_pyfunction!(stablecoin_infos, module)?)?;
    module.add_function(wrap_pyfunction!(uniswap_v2_pairs, module)?)?;
    module.add_function(wrap_pyfunction!(uniswap_v3_pairs, module)?)?;
    module.add_function(wrap_pyfunction!(sushiswap_pairs, module)?)?;
    Ok(())
}
