use std::ffi::CString;

use pyo3::ffi::c_str;
use pyo3::prelude::*;
use std::sync::OnceLock;

use crate::wrappers::{AsyncResult, ILoop, ThreadResult};

#[derive(Debug)]
pub struct PyConstructors {
    async_result_constructor: Py<PyAny>,
    thread_result_constructor: Py<PyAny>,
    iloop: ILoop,
}

impl PyConstructors {
    pub fn get<'py>(py: Python<'py>) -> &'static PyConstructors {
        static CONSTRUCTORS: OnceLock<PyConstructors> = OnceLock::new();
        CONSTRUCTORS.get_or_init(|| Self::with_py(py).expect("failed to initialize constructors!"))
    }

    /// Construct TaskLocals
    #[tracing::instrument(skip(py))]
    fn with_py(py: Python) -> PyResult<Self> {
        let gevent_hub = py.import("gevent.hub")?;
        let hub = gevent_hub.getattr("get_hub")?.call0()?;
        let iloop = hub.getattr("loop")?;

        let async_result_constructor = py.import("gevent.event")?.getattr("AsyncResult")?;
        let thread_result_module = PyModule::from_code(
            py,
            CString::new(include_str!("thread_result.py"))
                .unwrap()
                .as_c_str(),
            c_str!("thread_result.py"),
            c_str!("thread_result"),
        )?;
        let thread_result_constructor = thread_result_module.getattr("ThreadResult")?;
        Ok(Self {
            async_result_constructor: async_result_constructor.unbind(),
            thread_result_constructor: thread_result_constructor.unbind(),
            iloop: ILoop(iloop.unbind()),
        })
    }

    pub fn get_iloop<'p>(&self, py: Python<'p>) -> ILoop {
        self.iloop.clone_ref(py)
    }

    /// Call the AsyncResult constructor
    pub fn new_async_result<'p>(&self, py: Python<'p>) -> PyResult<AsyncResult> {
        let result = self.async_result_constructor.call0(py)?;
        Ok(AsyncResult(result))
    }

    /// Call the ThreadResult constructor
    pub fn new_thread_result<'p>(
        &self,
        py: Python<'p>,
        async_result: AsyncResult,
    ) -> PyResult<ThreadResult> {
        let result = self
            .thread_result_constructor
            .call1(py, (async_result.0,))?;
        Ok(ThreadResult(result))
    }
}
