use crate::{
    py_constructors::PyConstructors,
    wrappers::{AsyncResult, ThreadResult},
};
use std::marker::PhantomData;

use pyo3::prelude::*;

pub fn new_thread_result<T, E>() -> PyResult<(Sender<T, E>, Receiver<T>)> {
    Python::attach(|py| {
        let constructors = PyConstructors::get(py);

        let async_result = constructors.new_async_result(py)?;
        let thread_result = constructors.new_thread_result(py, async_result.clone_ref(py))?;

        let sender = Sender {
            thread_result,
            marker: PhantomData,
            is_completed: false,
        };
        let receiver = Receiver {
            async_result,
            marker: PhantomData,
        };

        Ok((sender, receiver))
    })
}

pub struct Sender<T, E> {
    thread_result: ThreadResult,
    marker: PhantomData<Result<T, E>>,
    is_completed: bool,
}

pub struct Receiver<T> {
    async_result: AsyncResult,
    marker: PhantomData<T>,
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

impl<T, E> Drop for Sender<T, E> {
    fn drop(&mut self) {
        if self.is_completed {
            return;
        }
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
}
