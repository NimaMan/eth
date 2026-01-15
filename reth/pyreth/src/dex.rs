use pyo3::prelude::*;
use reth_chain_query::dex::pool_types::{
    canonicalize_pool_type as rc_canonicalize, DEX_POOL_TYPES,
};

#[pyfunction]
pub fn dex_pool_types() -> Vec<String> {
    DEX_POOL_TYPES
        .iter()
        .map(|value| value.to_string())
        .collect()
}

#[pyfunction]
pub fn canonicalize_dex_pool_type(name: &str) -> Option<String> {
    rc_canonicalize(name).map(|s| s.to_string())
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(dex_pool_types, module)?)?;
    module.add_function(wrap_pyfunction!(canonicalize_dex_pool_type, module)?)?;
    Ok(())
}
