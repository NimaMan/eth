use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_asyncio_0_21 as pyo3_asyncio;
use tx_fund_flow_fundflownetwork::{
    FundFlowAnalyzer, NetworkBuilder, FundFlowNetwork,
    CytoscapeExporter, VisJsExporter
};
use tx_fund_flow_core_types::{EntityType};
use alloy_primitives::Address;
use std::str::FromStr;
use tokio::runtime::Runtime;
use std::sync::Arc;
use std::collections::HashMap;

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
        let db_url = database_url.unwrap_or_else(|| 
            std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eth_db".to_string())
        );
        
        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create Tokio runtime: {}", e)
            ))?;
            
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
        let address = Address::from_str(&seed_address)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid Ethereum address: {}", e)
            ))?;

        let db_url = self.database_url.clone();
        
        // Run async work in the runtime
        let result = self.runtime.block_on(async move {
            // Create database connection
            let db_pool = sqlx::PgPool::connect(&db_url).await
                .map_err(|e| format!("Database connection failed: {}", e))?;
            
            // Build the network using Layer 1 (graph exploration)
            let mut network = FundFlowNetwork::new();
            
            // TODO: Implement actual network building logic
            // This is a placeholder - integrate with actual fundflownetwork module
            
            // For now, create a simple example network
            network.add_node(address, HashMap::from([
                ("label".to_string(), "Seed Address".to_string()),
                ("entity_type".to_string(), "Unknown".to_string()),
            ]));
            
            Ok::<serde_json::Value, String>(network.to_cytoscape_json())
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        
        // Convert JSON to Python dict
        let py_dict = json_to_pyobject(py, &result)?;
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
        py: Python<'_>,
        network: &Bound<'_, PyDict>,
        address: String,
        depth: u32,
    ) -> PyResult<PyObject> {
        let addr = Address::from_str(&address)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid Ethereum address: {}", e)
            ))?;
            
        // TODO: Implement node expansion logic
        
        // For now, return the same network
        Ok(network.to_object(py))
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
    fn get_fund_flow_insights(
        &self,
        py: Python<'_>,
        address: String,
        lookback_blocks: u64,
    ) -> PyResult<PyObject> {
        let addr = Address::from_str(&address)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid Ethereum address: {}", e)
            ))?;
            
        let insights = PyDict::new(py);
        
        // TODO: Implement actual insights calculation
        insights.set_item("address", address)?;
        insights.set_item("upstream_sources", PyList::empty(py))?;
        insights.set_item("downstream_sinks", PyList::empty(py))?;
        insights.set_item("total_inflow_eth", 0.0)?;
        insights.set_item("total_outflow_eth", 0.0)?;
        insights.set_item("net_flow_eth", 0.0)?;
        
        Ok(insights.to_object(py))
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
            "visjs" => {
                // TODO: Convert to VisJS format
                Ok(network.to_object(py))
            },
            "graphml" => {
                // TODO: Convert to GraphML format
                let graphml = "<graphml><!-- Network data --></graphml>";
                Ok(graphml.to_object(py))
            },
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Unknown export format: {}. Use 'cytoscape', 'visjs', or 'graphml'", format)
            ))
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
        },
        serde_json::Value::String(s) => Ok(s.to_object(py)),
        serde_json::Value::Array(arr) => {
            let py_list = PyList::empty(py);
            for item in arr {
                py_list.append(json_to_pyobject(py, item)?)?;
            }
            Ok(py_list.to_object(py))
        },
        serde_json::Value::Object(map) => {
            let py_dict = PyDict::new(py);
            for (key, val) in map {
                py_dict.set_item(key, json_to_pyobject(py, val)?)?;
            }
            Ok(py_dict.to_object(py))
        }
    }
}

// Placeholder for FundFlowNetwork (should come from tx_fund_flow-fundflownetwork)
struct FundFlowNetwork {
    nodes: Vec<HashMap<String, String>>,
    edges: Vec<HashMap<String, String>>,
}

impl FundFlowNetwork {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
    
    fn add_node(&mut self, address: Address, metadata: HashMap<String, String>) {
        let mut node = metadata;
        node.insert("id".to_string(), format!("{:?}", address));
        self.nodes.push(node);
    }
    
    fn to_cytoscape_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes,
            "edges": self.edges,
            "stats": {
                "total_nodes": self.nodes.len(),
                "total_edges": self.edges.len(),
            }
        })
    }
}

/// Python module definition
#[pymodule]
fn fundflownetwork_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFundFlowNetworkBuilder>()?;
    
    // Add version info
    m.add("__version__", "0.1.0")?;
    m.add("__doc__", "High-performance fund flow network analysis for Ethereum")?;
    
    Ok(())
}