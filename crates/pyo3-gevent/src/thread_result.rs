use crate::{
    py_constructors::PyConstructors,
    wrappers::{AsyncResult, ThreadResult},
};
use std::{marker::PhantomData, sync::Arc, time::Duration};

use pyo3::{
    prelude::*,
    types::{PyDict, PyDictMethods},
};

pub fn new_thread_result<T, E>() -> PyResult<(Sender<T, E>, Receiver<T>)> {
    Python::attach(|py| {
        let constructors = PyConstructors::get(py);

        let async_result = constructors.new_async_result(py)?;
        let thread_result = constructors.new_thread_result(py, async_result.clone_ref(py))?;

        let shared_drop_guard = Arc::new(SharedDropGuard {
            thread_result: thread_result.clone_ref(py),
        });

        let sender = Sender {
            thread_result,
            marker: PhantomData,
            is_completed: false,
            _shared_drop_guard: Arc::clone(&shared_drop_guard),
        };
        let receiver = Receiver {
            async_result,
            marker: PhantomData,
            _shared_drop_guard: shared_drop_guard,
        };

        Ok((sender, receiver))
    })
}

/// Holds a ptr to the ThreadResult active and calls
/// `ThreadResult.destroy_in_main_thread` when both the sender and receiver are
/// dropped. As long as one of the two is active, the ThreadResult will not be
/// destroyed from the gevent Hub.
struct SharedDropGuard {
    thread_result: ThreadResult,
}

pub struct Sender<T, E> {
    thread_result: ThreadResult,
    marker: PhantomData<Result<T, E>>,
    is_completed: bool,
    _shared_drop_guard: Arc<SharedDropGuard>,
}

pub struct Receiver<T> {
    async_result: AsyncResult,
    marker: PhantomData<T>,
    _shared_drop_guard: Arc<SharedDropGuard>,
}

impl<T, E> Sender<T, E>
where
    for<'py> T: IntoPyObject<'py>,
    for<'py> E: IntoPyObject<'py>,
{
    pub fn complete_ok(self, value: T) -> PyResult<()> {
        Python::attach(|py| self.thread_result.0.call_method1(py, "set", (value,)))?;

        Ok(())
    }

    pub fn complete_err(self, err: E) -> PyResult<()> {
        Python::attach(|py| {
            self.thread_result
                .0
                .call_method1(py, "handle_error", (None::<Py<PyAny>>, err))
        })?;

        Ok(())
    }

    pub fn complete(mut self, result: Result<T, E>) -> PyResult<()> {
        self.is_completed = true;
        match result {
            Ok(x) => self.complete_ok(x),
            Err(e) => self.complete_err(e),
        }
    }
}

impl Drop for SharedDropGuard {
    fn drop(&mut self) {
        Python::attach(|py| {
            let py_constructors = PyConstructors::get(py);
            let iloop = py_constructors.get_iloop(py);
            iloop
                .0
                .call_method1(
                    py,
                    "run_callback_threadsafe",
                    (self
                        .thread_result
                        .0
                        .getattr(py, "destroy_in_main_thread")
                        .unwrap(),),
                )
                .unwrap();
        });
    }
}

impl<T> Receiver<T> {
    pub fn wait(self) -> PyResult<Py<PyAny>> {
        Python::attach(|py| self.async_result.0.call_method0(py, "get"))
    }

    pub fn wait_timeout(self, timeout: Duration) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            let kwargs = PyDict::new(py);
            kwargs.set_item("timeout", timeout.as_secs_f64())?;
            self.async_result
                .0
                .call_method(py, "get", (), Some(&kwargs))
        })
    }
}
