use alloy_primitives::{Address, U256};
use pyo3::exceptions::{PyNotImplementedError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::json;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tx_fund_flow_fundflownetwork::graph_discovery::{
    DiscoveryConfig, DiscoveryOutput, GraphExplorer,
};

/// Python wrapper for FundFlowNetwork builder
#[pyclass]
pub struct PyFundFlowNetworkBuilder {
    runtime: Arc<Runtime>,
    database_url: String,
}

#[pymethods]
impl PyFundFlowNetworkBuilder {
    #[new]
    #[pyo3(signature = (database_url=None))]
    fn new(database_url: Option<String>) -> PyResult<Self> {
        let db_url = database_url.unwrap_or_else(|| {
            std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://postgres:postgres@localhost:5432/eth_db".to_string()
            })
        });

        let runtime = Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create Tokio runtime: {}",
                e
            ))
        })?;

        Ok(Self {
            runtime: Arc::new(runtime),
            database_url: db_url,
        })
    }

    /// Build fund flow network from a seed address
    ///
    /// Args:
    ///     seed_address: Starting Ethereum address
    ///     max_depth: Maximum exploration depth (default: 3)
    ///     min_value_eth: Minimum ETH value to include (default: 0.1)
    ///     max_nodes: Maximum number of nodes (default: 500)
    ///     include_tokens: Include token transfers (default: true)
    ///     
    /// Returns:
    ///     Dictionary with network data in Cytoscape format
    #[pyo3(signature = (seed_address, max_depth=3, min_value_eth=0.1, max_nodes=500, include_tokens=true))]
    fn build_from_address(
        &self,
        py: Python<'_>,
        seed_address: String,
        max_depth: u32,
        min_value_eth: f64,
        max_nodes: usize,
        include_tokens: bool,
    ) -> PyResult<PyObject> {
        let address = parse_address(&seed_address)?;
        let min_value_wei = eth_to_wei(min_value_eth)?;
        if max_depth == 0 {
            return Err(PyValueError::new_err("max_depth must be at least 1"));
        }
        if max_nodes == 0 {
            return Err(PyValueError::new_err("max_nodes must be at least 1"));
        }

        let db_url = self.database_url.clone();
        let config = DiscoveryConfig {
            max_depth,
            max_nodes,
            min_value_wei,
            max_txs_per_address: 100,
            max_block_number: None,
        };

        let result = self
            .runtime
            .block_on(async move {
                let db_pool = sqlx::PgPool::connect(&db_url)
                    .await
                    .map_err(|e| format!("Database connection failed: {}", e))?;
                let explorer = GraphExplorer::new(db_pool);
                explorer
                    .explore(address, config, &HashSet::new())
                    .await
                    .map_err(|e| format!("Graph discovery failed: {}", e))
            })
            .map_err(PyRuntimeError::new_err)?;

        let py_dict = json_to_pyobject(py, &discovery_to_cytoscape_json(&result, include_tokens))?;
        Ok(py_dict)
    }

    /// Expand network from a specific node
    ///
    /// Args:
    ///     network: Current network dictionary
    ///     address: Address to expand from
    ///     depth: Additional depth to explore
    ///     
    /// Returns:
    ///     Updated network dictionary
    #[pyo3(signature = (network, address, depth=1))]
    fn expand_node(
        &self,
        network: &Bound<'_, PyDict>,
        address: String,
        depth: u32,
    ) -> PyResult<PyObject> {
        let _ = network;
        let _address = parse_address(&address)?;
        Err(PyNotImplementedError::new_err(format!(
            "incremental expansion is not wired to persisted discovery state yet; requested depth={}",
            depth
        )))
    }

    /// Get fund flow insights for an address
    ///
    /// Args:
    ///     address: Ethereum address to analyze
    ///     lookback_blocks: Number of blocks to look back
    ///     
    /// Returns:
    ///     Dictionary with upstream sources and downstream sinks
    #[pyo3(signature = (address, lookback_blocks=10000))]
    fn get_fund_flow_insights(&self, address: String, lookback_blocks: u64) -> PyResult<PyObject> {
        let _address = parse_address(&address)?;
        Err(PyNotImplementedError::new_err(format!(
            "fund-flow insight aggregation needs processed transaction flow extraction; requested lookback_blocks={}",
            lookback_blocks
        )))
    }

    /// Export network to different formats
    ///
    /// Args:
    ///     network: Network dictionary
    ///     format: Export format ("cytoscape", "visjs", "graphml")
    ///     
    /// Returns:
    ///     Formatted network data
    #[pyo3(signature = (network, format="cytoscape"))]
    fn export_network(
        &self,
        py: Python<'_>,
        network: &Bound<'_, PyDict>,
        format: &str,
    ) -> PyResult<PyObject> {
        match format {
            "cytoscape" => Ok(network.to_object(py)), // Already in Cytoscape format
            "visjs" => Err(PyNotImplementedError::new_err(
                "visjs export requires a typed FundFlowNetwork, not a Python dict",
            )),
            "graphml" => Err(PyNotImplementedError::new_err(
                "graphml export requires a typed FundFlowNetwork, not a Python dict",
            )),
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown export format: {}. Use 'cytoscape', 'visjs', or 'graphml'",
                format
            ))),
        }
    }
}

/// Helper to convert serde_json::Value to Python object
fn json_to_pyobject(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.to_object(py)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Ok(n.to_string().to_object(py))
            }
        }
        serde_json::Value::String(s) => Ok(s.to_object(py)),
        serde_json::Value::Array(arr) => {
            let py_list = PyList::empty_bound(py);
            for item in arr {
                py_list.append(json_to_pyobject(py, item)?)?;
            }
            Ok(py_list.to_object(py))
        }
        serde_json::Value::Object(map) => {
            let py_dict = PyDict::new_bound(py);
            for (key, val) in map {
                py_dict.set_item(key, json_to_pyobject(py, val)?)?;
            }
            Ok(py_dict.to_object(py))
        }
    }
}

fn parse_address(value: &str) -> PyResult<Address> {
    Address::from_str(value).map_err(|e| PyValueError::new_err(format!("Invalid address: {}", e)))
}

fn eth_to_wei(value: f64) -> PyResult<U256> {
    if !value.is_finite() || value < 0.0 {
        return Err(PyValueError::new_err(
            "min_value_eth must be a finite non-negative number",
        ));
    }

    let wei = value * 1_000_000_000_000_000_000_f64;
    if wei > u128::MAX as f64 {
        return Err(PyValueError::new_err("min_value_eth is too large"));
    }

    Ok(U256::from(wei.round() as u128))
}

fn discovery_to_cytoscape_json(
    output: &DiscoveryOutput,
    include_tokens_requested: bool,
) -> serde_json::Value {
    let mut elements = Vec::new();

    for node in output.graph.nodes.values() {
        let address = format!("{:?}", node.address);
        let label = node
            .name
            .as_deref()
            .or(node.entity_type.as_deref())
            .unwrap_or(&address);
        elements.push(json!({
            "data": {
                "id": address,
                "label": label,
                "address": address,
                "entity_type": node.entity_type,
                "is_contract": node.is_contract,
                "explored": node.explored,
                "exploration_depth": node.exploration_depth,
                "first_seen_block": node.first_seen_block,
            },
            "classes": if node.is_contract { "contract" } else { "eoa" },
        }));
    }

    for edge in &output.graph.edges {
        let source = format!("{:?}", edge.node1);
        let target = format!("{:?}", edge.node2);
        elements.push(json!({
            "data": {
                "id": format!("{:?}-{}-{}", edge.tx_hash, source, target),
                "source": source,
                "target": target,
                "tx_hash": format!("{:?}", edge.tx_hash),
                "value_wei": edge.value.to_string(),
                "block_number": edge.block_number,
            },
            "classes": "observed-transaction",
        }));
    }

    json!({
        "elements": elements,
        "layout": {
            "name": "cose",
            "animate": false,
        },
        "discovery": {
            "stats": output.discovery_stats,
            "stop_reason": output.stop_reason,
            "priority_transactions": output.priority_transactions,
            "include_tokens_requested": include_tokens_requested,
            "token_edges_supported": false,
        },
    })
}

/// Python module definition
#[pymodule]
fn fundflownetwork_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFundFlowNetworkBuilder>()?;

    // Add version info
    m.add("__version__", "0.1.0")?;
    m.add(
        "__doc__",
        "High-performance fund flow network analysis for Ethereum",
    )?;

    Ok(())
}
