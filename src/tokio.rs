use std::boxed::Box;
use std::fmt;
use std::pin::Pin;
use std::{future::Future, sync::Mutex};

use once_cell::{
    sync::{Lazy, OnceCell},
    unsync::OnceCell as UnsyncOnceCell,
};
use pyo3::{create_exception, prelude::*};
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
}

impl TaskLocals {
    /// At a minimum, TaskLocals must store the event loop.
    pub fn new(hub: &PyAny, iloop: &PyAny) -> Self {
        Self {
            hub: hub.into(),
            iloop: iloop.into(),
        }
    }

    /// Construct TaskLocals
    pub fn with_py(py: Python) -> PyResult<Self> {
        let gevent_hub = py.import("gevent.hub")?;
        let hub = gevent_hub.getattr("get_hub")?.call0()?;
        let iloop = hub.getattr("loop")?;
        Ok(Self::new(hub, iloop))
    }

    /// Get a reference to the hub
    pub fn hub<'p>(&self, py: Python<'p>) -> &'p PyAny {
        self.hub.clone().into_ref(py)
    }

    /// Get a reference to the event loop
    pub fn iloop<'p>(&self, py: Python<'p>) -> &'p PyAny {
        self.iloop.clone().into_ref(py)
    }
}

fn get_task_locals() -> Option<TaskLocals> {
    match TASK_LOCALS.try_with(|c| c.get().map(|locals| locals.clone())) {
        Ok(locals) => locals,
        Err(_) => None,
    }
}

fn get_current_locals(py: Python) -> PyResult<TaskLocals> {
    if let Some(locals) = get_task_locals() {
        Ok(locals)
    } else {
        Ok(TaskLocals::with_py(py)?)
    }
}

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

static TOKIO_BUILDER: Lazy<Mutex<Builder>> = Lazy::new(|| Mutex::new(multi_thread()));
static TOKIO_RUNTIME: OnceCell<Runtime> = OnceCell::new();

pub fn get_runtime<'a>() -> &'a Runtime {
    TOKIO_RUNTIME.get_or_init(|| {
        TOKIO_BUILDER
            .lock()
            .unwrap()
            .build()
            .expect("Unable to build Tokio runtime")
    })
}

fn create_async_result(py: Python) -> PyResult<&PyAny> {
    let async_result = py.import("gevent.event")?.getattr("AsyncResult")?.call0()?;
    Ok(async_result)
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

pub fn future_into_py<F, T>(py: Python, fut: F) -> PyResult<&PyAny>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + IntoPy<PyObject> + fmt::Debug,
{
    let locals = get_current_locals(py)?;
    let py_fut = create_async_result(py)?;

    let future_tx1 = PyObject::from(py_fut);
    let future_tx2 = future_tx1.clone();

    get_runtime().spawn(async move {
        let locals2 = locals.clone();
        let run_task = get_runtime()
            .spawn(async move {
                let result = scope(locals2.clone(), fut).await;
                println!("result: {:?}", result);
                Python::with_gil(move |py| {
                    let _ = set_result(
                        locals2.iloop(py),
                        future_tx1.as_ref(py),
                        result.map(|val| val.into_py(py)),
                    )
                    .map_err(dump_err(py));
                });
            })
            .await;

        if let Err(e) = run_task {
            if e.is_panic() {
                Python::with_gil(move |py| {
                    let exc = PyO3GeventError::new_err(e.to_string());
                    let _ = set_result(locals.iloop(py), future_tx2.as_ref(py), Err(exc))
                        .map_err(dump_err(py));
                });
            }
        }
    });

    Ok(py_fut)
}

pub fn future_into_greenlet<F, T>(py: Python, fut: F) -> PyResult<&PyAny>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + IntoPy<PyObject> + fmt::Debug,
{
    let locals = get_current_locals(py)?;
    let locals2 = locals.clone();
    let locals3 = locals.clone();
    let current_greenlet = py.import("gevent")?.getattr("getcurrent")?.call0()?;
    println!("current_greenlet: {:?}", current_greenlet);
    let greenlet_tx1 = PyObject::from(current_greenlet);
    let greenlet_tx2 = greenlet_tx1.clone();

    get_runtime().spawn(async move {
        let run_task = get_runtime()
            .spawn(async move {
                let result = scope(locals.clone(), fut).await;
                Python::with_gil(move |py| {
                    let _ = switch_back(
                        locals.iloop(py),
                        greenlet_tx1.as_ref(py),
                        result.map(|val| val.into_py(py)),
                    )
                    .map_err(dump_err(py));
                });
            })
            .await;

        if let Err(e) = run_task {
            if e.is_panic() {
                Python::with_gil(move |py| {
                    let exc = PyO3GeventError::new_err(e.to_string());
                    let _ = switch_back(locals2.iloop(py), greenlet_tx2.as_ref(py), Err(exc))
                        .map_err(dump_err(py));
                });
            }
        }
    });

    locals3.hub(py).call_method0("switch")
}
