use pyo3::{Py, PyAny, Python};

/// Wrapper around a ThreadResult
pub struct ThreadResult(pub(crate) Py<PyAny>);

/// Wrapper around a gevent.event.AsyncResult
pub struct AsyncResult(pub(crate) Py<PyAny>);

impl AsyncResult {
    pub fn clone_ref(&self, py: Python<'_>) -> Self {
        Self(self.0.clone_ref(py))
    }
}
