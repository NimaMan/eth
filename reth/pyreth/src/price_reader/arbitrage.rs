use pyo3::prelude::*;

/// Python wrapper for arbitrage detection across DEX and CEX
#[pyclass]
pub struct PyArbitrageDetector {
    // Will be implemented to detect arbitrage opportunities
}

#[pymethods]
impl PyArbitrageDetector {
    #[new]
    fn new() -> Self {
        PyArbitrageDetector {}
    }
}