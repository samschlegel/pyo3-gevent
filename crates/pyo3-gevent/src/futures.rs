use std::fmt;
use std::future::Future;

use pyo3::prelude::*;
use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

use crate::thread_result::{new_thread_result, Receiver};
// use crate::PyO3GeventError;

static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();
pub fn get_runtime<'a>() -> &'a Runtime {
    TOKIO_RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime!")
    })
}

fn dump_err(err: PyErr) {
    Python::attach(|py| {
        // We can't display Python exceptions via std::fmt::Display,
        // so print the error here manually.
        err.print_and_set_sys_last_vars(py);
    });
}

#[tracing::instrument(skip_all)]
pub fn future_into_py<F, T>(fut: F) -> PyResult<Receiver<T>>
where
    F: Future<Output = T> + Send + 'static,
    for<'py> T: 'static + Send + IntoPyObject<'py> + fmt::Debug,
{
    let (tx, rx) = new_thread_result::<T, ()>()?;

    get_runtime().spawn(async move {
        if let Err(e) = tx.complete_ok(fut.await) {
            dump_err(e);
        };
    });

    Ok(rx)
}
