pub mod timing;
pub mod tokio;

use pyo3::create_exception;
use pyo3::prelude::*;
use tracing::metadata::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

create_exception!(pyo3_gevent, PyO3GeventError, pyo3::exceptions::PyException);

/// Formats the sum of two numbers as string.
#[pyfunction]
#[tracing::instrument(skip(py))]
fn async_sum_as_string(py: Python, a: usize, b: usize) -> PyResult<&PyAny> {
    let result = tokio::future_into_py(py, async move {
        // ::tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok((a + b).to_string())
    })?;
    result.getattr("get")?.call0()
}

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(py: Python, a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

#[pyfunction]
fn init_tracing() -> PyResult<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::FULL)
        .init();
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn pyo3_gevent(py: Python, m: &PyModule) -> PyResult<()> {
    tokio::init(py);
    m.add("PyO3GeventError", py.get_type::<PyO3GeventError>())?;
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    m.add_function(wrap_pyfunction!(async_sum_as_string, m)?)?;
    m.add_function(wrap_pyfunction!(init_tracing, m)?)?;
    m.add_class::<timing::Timings>()?;
    Ok(())
}
