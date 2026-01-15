use pyo3::prelude::*;
use reth_chain_query::function_signatures::{
    CRITICAL_SCAM_FUNCTIONS, EVENT_TOPICS, EVENT_TOPICS_REVERSE, FUNCTION_SIGNATURES,
    UNISWAP_CONTRACTS,
};
use std::collections::HashMap;

#[pyfunction]
pub fn function_signatures() -> HashMap<String, String> {
    FUNCTION_SIGNATURES
        .iter()
        .map(|(selector, label)| (selector.clone(), (*label).to_string()))
        .collect()
}

#[pyfunction]
pub fn event_topics() -> HashMap<String, String> {
    EVENT_TOPICS
        .iter()
        .map(|(name, topic)| ((*name).to_string(), hex::encode(topic.as_slice())))
        .collect()
}

#[pyfunction]
pub fn event_topics_reverse() -> HashMap<String, String> {
    EVENT_TOPICS_REVERSE
        .iter()
        .map(|(topic, name)| (topic.clone(), (*name).to_string()))
        .collect()
}

#[pyfunction]
pub fn critical_scam_function_selectors() -> Vec<String> {
    CRITICAL_SCAM_FUNCTIONS.iter().cloned().collect()
}

#[pyfunction]
pub fn uniswap_contracts() -> HashMap<String, String> {
    UNISWAP_CONTRACTS
        .iter()
        .map(|(addr, label)| ((*addr).to_string(), (*label).to_string()))
        .collect()
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(function_signatures, module)?)?;
    module.add_function(wrap_pyfunction!(event_topics, module)?)?;
    module.add_function(wrap_pyfunction!(event_topics_reverse, module)?)?;
    module.add_function(wrap_pyfunction!(critical_scam_function_selectors, module)?)?;
    module.add_function(wrap_pyfunction!(uniswap_contracts, module)?)?;
    Ok(())
}
