use pyo3::{prelude::*, types::PyDict};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

#[pyclass]
pub struct Timings {
    pub map: HashMap<String, SystemTime>,
}

impl Timings {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn add_timing(&mut self, name: &str, time: SystemTime) {
        self.map.insert(name.to_string(), time);
    }
}

#[pymethods]
impl Timings {
    pub fn get_timings(&self, py: Python) -> PyResult<PyObject> {
        let d = PyDict::new(py);
        for (key, value) in &self.map {
            d.set_item(key, value.duration_since(UNIX_EPOCH).unwrap().as_nanos())?;
        }
        Ok(d.to_object(py))
    }
}
