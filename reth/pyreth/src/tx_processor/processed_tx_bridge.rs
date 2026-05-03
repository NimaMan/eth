use crate::tx_processor::py_processed_transaction::PyProcessedTransaction;
use alloy_primitives::U256;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyBytes, PyDict, PyFloat, PyList, PyLong, PyString, PyTuple};
use serde_json::{Map, Number, Value};
use tx_processor::ProcessedTransaction as RustProcessedTransaction;

/// Extract a Rust `ProcessedTransaction` from any Python object that represents it.
pub(crate) fn processed_transaction_from_py_object(
    prior: &Bound<'_, PyAny>,
) -> PyResult<RustProcessedTransaction> {
    if let Ok(py_processed) = prior.extract::<PyRef<PyProcessedTransaction>>() {
        return Ok(py_processed.to_processed_transaction());
    }

    // Dataclass instance → convert via dataclasses.asdict
    if prior.hasattr("__dataclass_fields__")? {
        let dataclasses = prior.py().import_bound("dataclasses")?;
        let dict_obj = dataclasses.call_method1("asdict", (prior,))?;
        return processed_transaction_from_mapping(&dict_obj);
    }

    // Objects exposing to_dict()
    if prior.hasattr("to_dict")? {
        let dict_obj = prior.call_method0("to_dict")?;
        return processed_transaction_from_mapping(&dict_obj);
    }

    // Raw mapping/dict
    processed_transaction_from_mapping(prior)
}

/// Convert a Python mapping/dict into a Rust `ProcessedTransaction`.
pub(crate) fn processed_transaction_from_py_dict(
    prior_dict: &Bound<'_, PyAny>,
) -> PyResult<RustProcessedTransaction> {
    processed_transaction_from_mapping(prior_dict)
}

pub(crate) fn processed_transactions_from_py_iterable(
    prior_iterable: &Bound<'_, PyAny>,
) -> PyResult<Vec<RustProcessedTransaction>> {
    if prior_iterable.is_none() {
        return Ok(Vec::new());
    }

    if prior_iterable.is_instance_of::<PyString>() {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "prior transactions must be provided as an iterable of processed transaction objects",
        ));
    }

    let iter = prior_iterable.iter().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "prior transactions must be an iterable of processed transaction objects",
        )
    })?;

    let mut transactions = Vec::new();
    for item in iter {
        let obj = item?;
        transactions.push(processed_transaction_from_py_object(&obj)?);
    }
    Ok(transactions)
}

fn processed_transaction_from_mapping(
    obj: &Bound<'_, PyAny>,
) -> PyResult<RustProcessedTransaction> {
    let py = obj.py();
    let owned_dict = to_owned_dict(py, obj)?;
    let mut path = Vec::new();
    let map = py_dict_to_json_map(owned_dict.bind(py), &mut path)?;
    let json_value = Value::Object(map.clone());
    let json_string = serde_json::to_string(&json_value).map_err(|err| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unable to serialize processed transaction payload: {err}"
        ))
    })?;

    let mut deserializer = serde_json::Deserializer::from_str(&json_string);
    match serde_path_to_error::deserialize::<_, RustProcessedTransaction>(&mut deserializer) {
        Ok(result) => Ok(result),
        Err(err) => {
            let path = err.path().to_string();
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid processed transaction dict at {path}: {}",
                err.inner()
            )))
        }
    }
}

fn to_owned_dict(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
    if let Ok(dict) = obj.downcast::<PyDict>() {
        return Ok(dict.clone().unbind());
    }

    if obj.hasattr("__dataclass_fields__")? {
        let dataclasses = py.import_bound("dataclasses")?;
        let dict_obj = dataclasses.call_method1("asdict", (obj,))?;
        return to_owned_dict(py, &dict_obj);
    }

    if obj.hasattr("to_dict")? {
        let dict_obj = obj.call_method0("to_dict")?;
        return to_owned_dict(py, &dict_obj);
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "prior transaction must be a dict, dataclass, or expose to_dict()",
    ))
}

fn py_dict_to_json_map(
    dict: &Bound<'_, PyDict>,
    path: &mut Vec<String>,
) -> PyResult<Map<String, Value>> {
    let mut result = Map::with_capacity(dict.len());
    for (key_obj, value_obj) in dict {
        let key: String = key_obj.extract()?;
        path.push(key.clone());
        let value = py_any_to_json(&value_obj, path)?;
        path.pop();
        result.insert(key, value);
    }
    Ok(result)
}

fn py_any_to_json(obj: &Bound<'_, PyAny>, path: &mut Vec<String>) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }

    if obj.is_instance_of::<PyBool>() {
        let value = obj.extract::<bool>()?;
        return Ok(Value::Bool(value));
    }

    if obj.is_instance_of::<PyLong>() {
        return py_long_to_value(obj.downcast::<PyLong>()?);
    }

    if obj.is_instance_of::<PyFloat>() {
        let value = obj.downcast::<PyFloat>()?.value();
        if let Some(number) = Number::from_f64(value) {
            return Ok(Value::Number(number));
        }
        return Ok(Value::String(value.to_string()));
    }

    if obj.is_instance_of::<PyString>() {
        let s = obj.extract::<String>()?;
        if matches!(path.last(), Some(key) if key == "input") {
            return hex_string_to_array(&s).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid hex input at {}",
                    path.join(".")
                ))
            });
        }
        return Ok(Value::String(s));
    }

    if obj.is_instance_of::<PyBytes>() {
        let bytes = obj.downcast::<PyBytes>()?.as_bytes();
        return Ok(Value::String(format!("0x{}", hex::encode(bytes))));
    }

    if obj.is_instance_of::<PyDict>() {
        let map = py_dict_to_json_map(obj.downcast::<PyDict>()?, path)?;
        return Ok(Value::Object(map));
    }

    if obj.is_instance_of::<PyList>() {
        let list = obj.downcast::<PyList>()?;
        let mut values = Vec::with_capacity(list.len());
        for (idx, item) in list.iter().enumerate() {
            path.push(format!("[{idx}]"));
            values.push(py_any_to_json(&item, path)?);
            path.pop();
        }
        return Ok(Value::Array(values));
    }

    if obj.is_instance_of::<PyTuple>() {
        let tuple = obj.downcast::<PyTuple>()?;
        let mut values = Vec::with_capacity(tuple.len());
        for (idx, item) in tuple.iter().enumerate() {
            path.push(format!("[{idx}]"));
            values.push(py_any_to_json(&item, path)?);
            path.pop();
        }
        return Ok(Value::Array(values));
    }

    if obj.hasattr("__dict__")? {
        let dict = obj.getattr("__dict__")?;
        if dict.is_instance_of::<PyDict>() {
            let map = py_dict_to_json_map(dict.downcast::<PyDict>()?, path)?;
            return Ok(Value::Object(map));
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Unsupported value type in processed transaction payload",
    ))
}

fn py_long_to_value(long: &Bound<'_, PyLong>) -> PyResult<Value> {
    if let Ok(value) = long.extract::<i64>() {
        return Ok(Value::Number(Number::from(value)));
    }
    if let Ok(value) = long.extract::<u64>() {
        return Ok(Value::Number(Number::from(value)));
    }

    let decimal = long.str()?.extract::<String>()?;
    if decimal.starts_with('-') {
        return Ok(Value::String(decimal));
    }

    let u256 = U256::from_str_radix(&decimal, 10).map_err(|err| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unable to convert integer '{decimal}' into U256: {err}"
        ))
    })?;
    Ok(Value::String(format!("{:#x}", u256)))
}

fn hex_string_to_array(value: &str) -> Option<Value> {
    let clean = value.trim_start_matches("0x");
    if clean.is_empty() {
        return Some(Value::Array(Vec::new()));
    }
    if clean.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(clean.len() / 2);
    for chunk in clean.as_bytes().chunks(2) {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        bytes.push(Value::Number(Number::from((hi << 4 | lo) as u8)));
    }
    Some(Value::Array(bytes))
}
