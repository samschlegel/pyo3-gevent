use pyo3::{create_exception, prelude::*};

create_exception!(pyo3_gevent, PyO3GeventError, pyo3::exceptions::PyException);

#[pymodule]
mod pyo3_gevent_example {
    use std::thread::{self, spawn};
    use std::time::Duration;

    use pyo3::prelude::*;
    use pyo3_gevent::futures::{self, get_runtime};
    use pyo3_gevent::thread_result::new_thread_result;
    use tracing::metadata::LevelFilter;
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::fmt::format::FmtSpan;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add(
            "PyO3GeventError",
            m.py().get_type::<crate::PyO3GeventError>(),
        )?;
        Ok(())
    }

    /// Formats the sum of two numbers as string.
    #[pyfunction]
    #[tracing::instrument]
    fn async_sum_as_string(a: usize, b: usize) -> PyResult<Py<PyAny>> {
        let result = futures::future_into_py(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            (a + b).to_string()
        })?;

        result.wait()
    }

    #[pyfunction]
    fn thread_result_ready_immediately() -> PyResult<()> {
        let (tx, rx) = new_thread_result::<(), ()>()?;
        tx.complete_ok(())?;
        rx.wait()?;
        // println!("did one iter!");

        Ok(())
    }

    #[pyfunction]
    fn thread_result_os_thread() -> PyResult<()> {
        let (tx, rx) = new_thread_result::<(), ()>()?;
        thread::spawn(move || {
            tx.complete_ok(()).unwrap();
        });
        rx.wait()?;

        Ok(())
    }

    #[pyfunction]
    fn thread_result_tokio_task() -> PyResult<()> {
        let (tx, rx) = new_thread_result::<(), ()>()?;
        get_runtime().spawn(async move {
            tx.complete_ok(()).unwrap();
        });
        rx.wait()?;

        Ok(())
    }

    #[pyfunction]
    fn sleep_thread(id: u8, sleep_ms: u64) -> PyResult<Py<PyAny>> {
        let (tx, rx) = new_thread_result::<String, ()>()?;
        spawn(move || {
            println!("{id}: sleep {sleep_ms}ms");
            std::thread::sleep(Duration::from_millis(sleep_ms));
            println!("{id}: done!");
            tx.complete_ok(format!("{} complete via thread::sleep!", id))
                .expect("failed to send!");
        });

        rx.wait()
    }

    #[pyfunction]
    fn sleep_tokio(id: u8, sleep_ms: u64) -> PyResult<Py<PyAny>> {
        let (tx, rx) = new_thread_result::<String, ()>()?;
        get_runtime().spawn(async move {
            println!("{id}: sleep {sleep_ms}ms");
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            println!("{id}: done!");
            tx.complete_ok(format!("{} complete via tokio!", id))
                .expect("failed to send!");
        });

        rx.wait()
    }

    /// Formats the sum of two numbers as string.
    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
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
}
