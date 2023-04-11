use std::boxed::Box;
use std::fmt;
use std::pin::Pin;
use std::time::UNIX_EPOCH;
use std::{future::Future, sync::Mutex};

use once_cell::{
    sync::{Lazy, OnceCell},
    unsync::OnceCell as UnsyncOnceCell,
};
use pyo3::prelude::*;
use tokio::runtime::{self, Builder, Runtime};

use crate::PyO3GeventError;

fn multi_thread() -> Builder {
    let mut builder = runtime::Builder::new_multi_thread();
    builder.enable_all();
    builder
}

#[derive(Debug, Clone)]
struct TaskLocals {
    hub: PyObject,
    iloop: PyObject,
    async_result: PyObject,
    thread_result: PyObject,
}

impl TaskLocals {
    /// At a minimum, TaskLocals must store the event loop.
    pub fn new(hub: &PyAny, iloop: &PyAny, async_result: &PyAny, thread_result: &PyAny) -> Self {
        Self {
            hub: hub.into(),
            iloop: iloop.into(),
            async_result: async_result.into(),
            thread_result: thread_result.into(),
        }
    }

    /// Construct TaskLocals
    #[tracing::instrument(skip(py))]
    pub fn with_py(py: Python) -> PyResult<Self> {
        let gevent_hub = py.import("gevent.hub")?;
        let hub = gevent_hub.getattr("get_hub")?.call0()?;
        let iloop = hub.getattr("loop")?;
        let async_result = py.import("gevent.event")?.getattr("AsyncResult")?;
        let thread_result_module = PyModule::from_code(
            py,
            include_str!("thread_result.py"),
            "thread_result.py",
            "thread_result",
        )?;
        let tr = thread_result_module.getattr("ThreadResult")?;
        Ok(Self::new(hub, iloop, async_result, tr))
    }

    /// Get a reference to the hub
    pub fn hub<'p>(&self, py: Python<'p>) -> &'p PyAny {
        self.hub.clone().into_ref(py)
    }

    /// Get a reference to the event loop
    pub fn iloop<'p>(&self, py: Python<'p>) -> &'p PyAny {
        self.iloop.clone().into_ref(py)
    }

    /// Get a reference to the event loop
    pub fn async_result<'p>(&self, py: Python<'p>) -> &'p PyAny {
        self.async_result.clone().into_ref(py)
    }

    /// Get a reference to the event loop
    pub fn thread_result<'p>(&self, py: Python<'p>) -> &'p PyAny {
        self.thread_result.clone().into_ref(py)
    }
}

#[tracing::instrument]
fn get_task_locals() -> Option<TaskLocals> {
    match TASK_LOCALS.try_with(|c| c.get().map(|locals| locals.clone())) {
        Ok(locals) => locals,
        Err(e) => None,
    }
}

#[tracing::instrument(skip(py))]
fn get_current_locals(py: Python) -> PyResult<TaskLocals> {
    // if let Some(locals) = get_task_locals() {
    //     Ok(locals)
    // } else {
    Ok(THREAD_LOCALS
        .get()
        .expect("Thread locals not initialized")
        .clone())
    // }
}

static THREAD_LOCALS: OnceCell<TaskLocals> = OnceCell::new();

tokio::task_local! {
    static TASK_LOCALS: UnsyncOnceCell<TaskLocals>;
}

fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
where
    F: Future<Output = R> + Send + 'static,
{
    let cell = UnsyncOnceCell::new();
    cell.set(locals).unwrap();

    Box::pin(TASK_LOCALS.scope(cell, fut))
}

static TOKIO_RUNTIME: OnceCell<Runtime> = OnceCell::new();

pub fn init(py: Python) -> PyResult<()> {
    let mut builder = runtime::Builder::new_multi_thread();
    builder.enable_all();
    TOKIO_RUNTIME
        .set(builder.build()?)
        .expect("Library already initialized");
    THREAD_LOCALS
        .set(TaskLocals::with_py(py)?)
        .expect("Library already initialized");
    Ok(())
}

pub fn get_runtime<'a>() -> &'a Runtime {
    TOKIO_RUNTIME.get().expect("Library not initialized")
}

fn create_async_result<'a, 'b>(py: Python<'a>, locals: &'b TaskLocals) -> PyResult<&'a PyAny> {
    let async_result = locals.async_result(py).call0()?;
    Ok(async_result)
}

fn create_waiter(py: Python) -> PyResult<&PyAny> {
    let waiter = py.import("gevent.hub")?.getattr("Waiter")?.call0()?;
    Ok(waiter)
}

fn set_result(iloop: &PyAny, future: &PyAny, result: PyResult<PyObject>) -> PyResult<()> {
    println!("set_result: {:?}", result);
    let py = iloop.py();
    let (complete, val) = match result {
        // Ok(val) => (future.getattr("set")?, val.into_py(py)),
        Ok(val) => (future.getattr("set")?, val.into_py(py)),
        Err(e) => (future.getattr("set_exception")?, e.to_object(py)),
    };

    iloop
        .getattr("run_callback_threadsafe")?
        .call1((complete, val))?;

    Ok(())
}

fn set_result_waiter(iloop: &PyAny, waiter: &PyAny, result: PyResult<PyObject>) -> PyResult<()> {
    println!("set_result_waiter: {:?}", result);
    let py = iloop.py();
    let (complete, val) = match result {
        // Ok(val) => (future.getattr("set")?, val.into_py(py)),
        Ok(val) => (waiter.getattr("switch")?, val.into_py(py)),
        Err(e) => (waiter.getattr("throw")?, e.to_object(py)),
    };

    iloop
        .getattr("run_callback_threadsafe")?
        .call1((complete, val))?;

    Ok(())
}

fn set_result_taskresult(task_result: &PyAny, result: PyResult<PyObject>) -> PyResult<()> {
    let py = task_result.py();
    match result {
        // Ok(val) => (future.getattr("set")?, val.into_py(py)),
        Ok(val) => {
            task_result.getattr("set")?.call1((val.into_py(py),))?;
        }
        Err(e) => {
            task_result
                .getattr("handle_error")?
                .call1((None::<PyObject>, e.to_object(py)))?;
        }
    };

    Ok(())
}

fn switch_back(iloop: &PyAny, greenlet: &PyAny, result: PyResult<PyObject>) -> PyResult<()> {
    let py = iloop.py();
    let (complete, val) = match result {
        Ok(val) => (greenlet.getattr("switch")?, val.into_py(py)),
        Err(e) => (greenlet.getattr("throw")?, e.to_object(py)),
    };

    iloop
        .getattr("run_callback_threadsafe")?
        .call1((complete, val))?;

    Ok(())
}

fn dump_err(py: Python<'_>) -> impl FnOnce(PyErr) + '_ {
    move |e| {
        // We can't display Python exceptions via std::fmt::Display,
        // so print the error here manually.
        e.print_and_set_sys_last_vars(py);
    }
}

#[tracing::instrument(skip_all)]
pub fn future_into_py<F, T>(py: Python, fut: F) -> PyResult<&PyAny>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + IntoPy<PyObject> + fmt::Debug,
{
    let locals = get_current_locals(py)?;
    let locals2 = locals.clone();

    let py_result = create_async_result(py, &locals)?;
    let tr = locals
        .thread_result(py)
        .call1((py_result, locals.hub(py)))?;

    let tr_tx1 = PyObject::from(tr);
    let tr_tx2 = tr_tx1.clone();

    get_runtime().spawn(scope(locals, async move {
        let run_task = get_runtime()
            .spawn(async move {
                let result = scope(locals2.clone(), fut).await;
                Python::with_gil(move |py| {
                    let _ =
                        set_result_taskresult(tr_tx1.as_ref(py), result.map(|val| val.into_py(py)))
                            .map_err(dump_err(py));
                });
            })
            .await;

        if let Err(e) = run_task {
            if e.is_panic() {
                Python::with_gil(move |py| {
                    let exc = PyO3GeventError::new_err(e.to_string());
                    let _ =
                        set_result_taskresult(tr_tx2.as_ref(py), Err(exc)).map_err(dump_err(py));
                });
            }
        }
    }));

    Ok(py_result)
}
