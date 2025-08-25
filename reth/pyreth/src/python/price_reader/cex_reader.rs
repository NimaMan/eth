use pyo3::prelude::*;

/// Python wrapper for CEX WebSocket price feeds
#[pyclass]
pub struct PyCexReader {
    // Will be implemented to expose CEX WebSocket functionality
}

#[pymethods]
impl PyCexReader {
    #[new]
    fn new() -> Self {
        PyCexReader {}
    }
}