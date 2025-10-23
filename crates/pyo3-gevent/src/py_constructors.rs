use std::ffi::CString;

use pyo3::ffi::c_str;
use pyo3::prelude::*;
use std::sync::OnceLock;

use crate::wrappers::{AsyncResult, ThreadResult};

#[derive(Debug)]
pub struct PyConstructors {
    async_result: Py<PyAny>,
    thread_result: Py<PyAny>,
}

impl PyConstructors {
    pub fn get<'py>(py: Python<'py>) -> &'static PyConstructors {
        static CONSTRUCTORS: OnceLock<PyConstructors> = OnceLock::new();
        CONSTRUCTORS.get_or_init(|| Self::with_py(py).expect("failed to initialize constructors!"))
    }

    fn new(async_result: Py<PyAny>, thread_result: Py<PyAny>) -> Self {
        Self {
            async_result,
            thread_result,
        }
    }

    /// Construct TaskLocals
    #[tracing::instrument(skip(py))]
    fn with_py(py: Python) -> PyResult<Self> {
        let async_result = py.import("gevent.event")?.getattr("AsyncResult")?;
        let thread_result_module = PyModule::from_code(
            py,
            CString::new(include_str!("thread_result.py"))
                .unwrap()
                .as_c_str(),
            c_str!("thread_result.py"),
            c_str!("thread_result"),
        )?;
        let tr = thread_result_module.getattr("ThreadResult")?;
        Ok(Self::new(async_result.unbind(), tr.unbind()))
    }

    /// Call the AsyncResult constructor
    pub fn new_async_result<'p>(&self, py: Python<'p>) -> PyResult<AsyncResult> {
        let result = self.async_result.call0(py)?;
        Ok(AsyncResult(result))
    }

    /// Call the ThreadResult constructor
    pub fn new_thread_result<'p>(
        &self,
        py: Python<'p>,
        async_result: AsyncResult,
    ) -> PyResult<ThreadResult> {
        let result = self.thread_result.call1(py, (async_result.0,))?;
        Ok(ThreadResult(result))
    }
}
