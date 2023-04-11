pub mod tokio;

use pyo3::create_exception;
use pyo3::prelude::*;

create_exception!(pyo3_gevent, PyO3GeventError, pyo3::exceptions::PyException);

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(py: Python, a: usize, b: usize) -> PyResult<&PyAny> {
    tokio::future_into_py(py, async move {
        ::tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok((a + b).to_string())
    })
}

/// A Python module implemented in Rust.
#[pymodule]
fn pyo3_gevent(py: Python, m: &PyModule) -> PyResult<()> {
    m.add("PyO3GeventError", py.get_type::<PyO3GeventError>())?;
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}
