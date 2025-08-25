/// Python bindings for entities module
/// 
/// Provides access to stablecoin, CEX, and ETF analysis functionality

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use std::str::FromStr;
use alloy_primitives::Address;

use reth_chain_query::entities::{
    common::{identify_entity_type, EntityType, TimePeriod},
    stablecoins::{
        StablecoinMarketAnalyzer, StablecoinSupplyTracker,
        is_stablecoin, get_stablecoin_by_address, STABLECOINS,
    },
    cex::{
        CexBalanceTracker, CexFlowAnalyzer,
        is_cex_address, get_cex_by_address, CEX_ADDRESS_COUNT,
    },
    etfs::{
        EtfHoldingsTracker, EtfFlowAnalyzer,
        is_etf_address, get_etf_by_address, ETF_ADDRESS_COUNT,
    },
};

use super::main::PyChainQuery;

// Entity identification functions

/// Identify entity type for an address
pub fn identify_entity(address: &str) -> PyResult<String> {
    let addr = Address::from_str(address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    
    let entity_type = identify_entity_type(addr);
    
    Ok(match entity_type {
        EntityType::Stablecoin => "stablecoin".to_string(),
        EntityType::CEX => "cex".to_string(),
        EntityType::ETF => "etf".to_string(),
        EntityType::Unknown => "unknown".to_string(),
    })
}

/// Check if address is a stablecoin
pub fn is_stablecoin_address(address: &str) -> PyResult<bool> {
    let addr = Address::from_str(address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    Ok(is_stablecoin(addr))
}

/// Check if address is a CEX address
pub fn is_cex(address: &str) -> PyResult<bool> {
    let addr = Address::from_str(address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    Ok(is_cex_address(addr))
}

/// Check if address is an ETF address
pub fn is_etf(address: &str) -> PyResult<bool> {
    let addr = Address::from_str(address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    Ok(is_etf_address(addr))
}

// Entity information functions

/// Get stablecoin information
pub fn get_stablecoin_info(py: Python, address: &str) -> PyResult<Option<Py<PyDict>>> {
    let addr = Address::from_str(address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    
    if let Some(info) = get_stablecoin_by_address(addr) {
        let dict = PyDict::new(py);
        dict.set_item("address", format!("{:?}", info.address))?;
        dict.set_item("symbol", info.symbol)?;
        dict.set_item("decimals", info.decimals)?;
        dict.set_item("unit", info.unit)?;
        Ok(Some(dict.into()))
    } else {
        Ok(None)
    }
}

/// Get CEX information
pub fn get_cex_info(py: Python, address: &str) -> PyResult<Option<Py<PyDict>>> {
    let addr = Address::from_str(address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    
    if let Some(info) = get_cex_by_address(addr) {
        let dict = PyDict::new(py);
        dict.set_item("address", format!("{:?}", info.address))?;
        dict.set_item("name", info.name)?;
        dict.set_item("exchange", info.exchange)?;
        Ok(Some(dict.into()))
    } else {
        Ok(None)
    }
}

/// Get ETF information
pub fn get_etf_info(py: Python, address: &str) -> PyResult<Option<Py<PyDict>>> {
    let addr = Address::from_str(address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    
    if let Some(info) = get_etf_by_address(addr) {
        let dict = PyDict::new(py);
        dict.set_item("address", format!("{:?}", info.address))?;
        dict.set_item("name", info.name)?;
        dict.set_item("provider", info.provider)?;
        Ok(Some(dict.into()))
    } else {
        Ok(None)
    }
}

// Stablecoin market analysis

/// Analyze stablecoin market share
pub fn analyze_stablecoin_market_share(
    chain_query: &PyChainQuery,
    py: Python,
    block_number: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let analyzer = StablecoinMarketAnalyzer::new(chain_query.chain_query().clone());
    
    let analysis = chain_query.runtime().block_on(async {
        analyzer.analyze_market(block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let dict = PyDict::new(py);
    dict.set_item("block_number", analysis.block_number)?;
    dict.set_item("total_market_supply", analysis.total_market_supply)?;
    dict.set_item("top_3_concentration", analysis.top_3_concentration)?;
    dict.set_item("top_5_concentration", analysis.top_5_concentration)?;
    dict.set_item("herfindahl_index", analysis.herfindahl_index)?;
    
    // Add stablecoins list
    let stablecoins_list = pyo3::types::PyList::empty(py);
    for coin in analysis.stablecoins {
        let coin_dict = PyDict::new(py);
        coin_dict.set_item("symbol", coin.info.symbol)?;
        coin_dict.set_item("unit", coin.info.unit)?;
        coin_dict.set_item("total_supply", coin.total_supply_formatted)?;
        coin_dict.set_item("market_share_percent", coin.market_share_percent)?;
        coin_dict.set_item("rank", coin.rank)?;
        coin_dict.set_item("unit", coin.info.unit)?;
        stablecoins_list.append(coin_dict)?;
    }
    dict.set_item("stablecoins", stablecoins_list)?;
    
    Ok(dict.into())
}

/// Analyze stablecoin market by currency unit
pub fn analyze_stablecoin_market_by_unit(
    chain_query: &PyChainQuery,
    py: Python,
    block_number: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let analyzer = StablecoinMarketAnalyzer::new(chain_query.chain_query().clone());
    
    let analysis = chain_query.runtime().block_on(async {
        analyzer.analyze_market_by_unit(block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let dict = PyDict::new(py);
    dict.set_item("block_number", analysis.block_number)?;
    
    // Add units dictionary
    let units_dict = PyDict::new(py);
    for (unit_name, unit_data) in analysis.units {
        let unit_dict = PyDict::new(py);
        unit_dict.set_item("unit_name", &unit_name)?;
        unit_dict.set_item("total_supply_in_unit", unit_data.total_supply_in_unit)?;
        
        // Add tokens list for this unit
        let tokens_list = pyo3::types::PyList::empty(py);
        for token in unit_data.tokens {
            let token_dict = PyDict::new(py);
            token_dict.set_item("symbol", token.info.symbol)?;
            token_dict.set_item("unit", token.info.unit)?;
            token_dict.set_item("total_supply", token.total_supply_formatted)?;
            token_dict.set_item("market_share_percent", token.market_share_percent)?;
            token_dict.set_item("rank", token.rank)?;
            tokens_list.append(token_dict)?;
        }
        unit_dict.set_item("tokens", tokens_list)?;
        
        units_dict.set_item(unit_name, unit_dict)?;
    }
    dict.set_item("units", units_dict)?;
    
    Ok(dict.into())
}

/// Get top N stablecoins
pub fn get_top_stablecoins(
    chain_query: &PyChainQuery,
    py: Python,
    n: usize,
    block_number: Option<u64>,
) -> PyResult<Py<pyo3::types::PyList>> {
    let analyzer = StablecoinMarketAnalyzer::new(chain_query.chain_query().clone());
    
    let top_coins = chain_query.runtime().block_on(async {
        analyzer.get_top_stablecoins(n, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let list = pyo3::types::PyList::empty(py);
    for coin in top_coins {
        let coin_dict = PyDict::new(py);
        coin_dict.set_item("symbol", coin.info.symbol)?;
        coin_dict.set_item("unit", coin.info.unit)?;
        coin_dict.set_item("total_supply", coin.total_supply_formatted)?;
        coin_dict.set_item("market_share_percent", coin.market_share_percent)?;
        coin_dict.set_item("rank", coin.rank)?;
        coin_dict.set_item("unit", coin.info.unit)?;
        list.append(coin_dict)?;
    }
    
    Ok(list.into())
}

// CEX balance tracking

/// Get CEX balances
pub fn get_cex_balances(
    chain_query: &PyChainQuery,
    py: Python,
    block_number: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let tracker = CexBalanceTracker::new(chain_query.chain_query().clone());
    
    let balances = chain_query.runtime().block_on(async {
        tracker.get_all_cex_eth_balances(block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let dict = PyDict::new(py);
    dict.set_item("block_number", balances.block_number)?;
    dict.set_item("total_eth_in_exchanges", balances.total_cex_eth_formatted)?;
    dict.set_item("exchange_count", balances.exchanges.len())?;
    
    // Add exchanges list
    let exchanges_list = pyo3::types::PyList::empty(py);
    for exchange in balances.exchanges {
        let exchange_dict = PyDict::new(py);
        exchange_dict.set_item("name", exchange.name)?;
        exchange_dict.set_item("total_balance", exchange.total_eth_formatted)?;
        exchange_dict.set_item("address_count", exchange.address_count)?;
        exchanges_list.append(exchange_dict)?;
    }
    dict.set_item("exchanges", exchanges_list)?;
    
    Ok(dict.into())
}

// ETF holdings tracking

/// Get ETF holdings
pub fn get_etf_holdings(
    chain_query: &PyChainQuery,
    py: Python,
    block_number: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let tracker = EtfHoldingsTracker::new(chain_query.chain_query().clone());
    
    let holdings = chain_query.runtime().block_on(async {
        tracker.get_all_etf_holdings(block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let dict = PyDict::new(py);
    dict.set_item("block_number", holdings.block_number)?;
    dict.set_item("total_eth_in_etfs", holdings.total_etf_eth_formatted)?;
    dict.set_item("provider_count", holdings.providers.len())?;
    
    // Add providers list
    let providers_list = pyo3::types::PyList::empty(py);
    for provider in holdings.providers {
        let provider_dict = PyDict::new(py);
        provider_dict.set_item("name", provider.name)?;
        provider_dict.set_item("total_balance", provider.total_eth_formatted)?;
        provider_dict.set_item("address_count", provider.address_count)?;
        provider_dict.set_item("market_share", provider.market_share_percent)?;
        providers_list.append(provider_dict)?;
    }
    dict.set_item("providers", providers_list)?;
    
    Ok(dict.into())
}

// Entity statistics

/// Get entity statistics
pub fn get_entity_stats(py: Python) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("stablecoin_count", STABLECOINS.len())?;
    dict.set_item("cex_address_count", CEX_ADDRESS_COUNT)?;
    dict.set_item("etf_address_count", ETF_ADDRESS_COUNT)?;
    dict.set_item("total_tracked", STABLECOINS.len() + CEX_ADDRESS_COUNT + ETF_ADDRESS_COUNT)?;
    Ok(dict.into())
}

// Flow calculation methods

/// Calculate ETF flows between blocks
pub fn calculate_etf_flows_between_blocks(
    chain_query: &PyChainQuery,
    py: Python,
    from_block: u64,
    to_block: u64,
) -> PyResult<Py<PyDict>> {
    let analyzer = EtfFlowAnalyzer::new(chain_query.chain_query().clone());
    
    let flow_summary = chain_query.runtime().block_on(async {
        analyzer.calculate_flows_between_blocks(from_block, to_block).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let dict = PyDict::new(py);
    dict.set_item("from_block", flow_summary.from_block)?;
    dict.set_item("to_block", flow_summary.to_block)?;
    dict.set_item("total_inflow_eth", flow_summary.total_inflow_eth)?;
    dict.set_item("total_outflow_eth", flow_summary.total_outflow_eth)?;
    dict.set_item("total_net_flow_eth", flow_summary.total_net_flow_eth)?;
    
    // Add provider flows
    let provider_flows_dict = PyDict::new(py);
    for (provider_name, flow_data) in flow_summary.provider_flows {
        let flow_dict = PyDict::new(py);
        flow_dict.set_item("provider", flow_data.provider)?;
        flow_dict.set_item("inflow_eth", flow_data.inflow_eth)?;
        flow_dict.set_item("outflow_eth", flow_data.outflow_eth)?;
        flow_dict.set_item("net_flow_eth", flow_data.net_flow_eth)?;
        flow_dict.set_item("transfer_count", flow_data.transfer_count)?;
        flow_dict.set_item("unique_addresses", flow_data.unique_addresses)?;
        provider_flows_dict.set_item(provider_name, flow_dict)?;
    }
    dict.set_item("provider_flows", provider_flows_dict)?;
    
    Ok(dict.into())
}

/// Calculate CEX flows between blocks
pub fn calculate_cex_flows_between_blocks(
    chain_query: &PyChainQuery,
    py: Python,
    from_block: u64,
    to_block: u64,
) -> PyResult<Py<PyDict>> {
    let analyzer = CexFlowAnalyzer::new(chain_query.chain_query().clone());
    
    let flow_summary = chain_query.runtime().block_on(async {
        analyzer.calculate_flows_between_blocks(from_block, to_block).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let dict = PyDict::new(py);
    dict.set_item("from_block", flow_summary.from_block)?;
    dict.set_item("to_block", flow_summary.to_block)?;
    dict.set_item("total_inflow_eth", flow_summary.total_inflow_eth)?;
    dict.set_item("total_outflow_eth", flow_summary.total_outflow_eth)?;
    dict.set_item("total_net_flow_eth", flow_summary.total_net_flow_eth)?;
    
    // Add exchange flows
    let exchange_flows_dict = PyDict::new(py);
    for (exchange_name, flow_data) in flow_summary.exchange_flows {
        let flow_dict = PyDict::new(py);
        flow_dict.set_item("exchange", flow_data.exchange)?;
        flow_dict.set_item("inflow_eth", flow_data.inflow_eth)?;
        flow_dict.set_item("outflow_eth", flow_data.outflow_eth)?;
        flow_dict.set_item("net_flow_eth", flow_data.net_flow_eth)?;
        flow_dict.set_item("transfer_count", flow_data.transfer_count)?;
        flow_dict.set_item("unique_addresses", flow_data.unique_addresses)?;
        exchange_flows_dict.set_item(exchange_name, flow_dict)?;
    }
    dict.set_item("exchange_flows", exchange_flows_dict)?;
    
    Ok(dict.into())
}

/// Calculate stablecoin supply changes between blocks
pub fn calculate_stablecoin_supply_changes_between_blocks(
    chain_query: &PyChainQuery,
    py: Python,
    from_block: u64,
    to_block: u64,
) -> PyResult<Py<PyDict>> {
    let tracker = StablecoinSupplyTracker::new(chain_query.chain_query().clone());
    
    let supply_summary = chain_query.runtime().block_on(async {
        tracker.calculate_supply_changes_between_blocks(from_block, to_block).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let dict = PyDict::new(py);
    dict.set_item("from_block", supply_summary.from_block)?;
    dict.set_item("to_block", supply_summary.to_block)?;
    dict.set_item("total_minted", supply_summary.total_minted)?;
    dict.set_item("total_burned", supply_summary.total_burned)?;
    dict.set_item("total_net_change", supply_summary.total_net_change)?;
    
    // Add token changes
    let token_changes_dict = PyDict::new(py);
    for (token_symbol, change_data) in supply_summary.token_changes {
        let change_dict = PyDict::new(py);
        change_dict.set_item("token_symbol", change_data.token_symbol)?;
        change_dict.set_item("token_address", format!("{:?}", change_data.token_address))?;
        change_dict.set_item("old_supply", change_data.old_supply)?;
        change_dict.set_item("new_supply", change_data.new_supply)?;
        change_dict.set_item("minted", change_data.minted)?;
        change_dict.set_item("burned", change_data.burned)?;
        change_dict.set_item("net_change", change_data.net_change)?;
        change_dict.set_item("percent_change", change_data.percent_change)?;
        token_changes_dict.set_item(token_symbol, change_dict)?;
    }
    dict.set_item("token_changes", token_changes_dict)?;
    
    Ok(dict.into())
}