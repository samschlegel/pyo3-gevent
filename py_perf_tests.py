from gevent.event import AsyncResult
import gevent
from timeit import timeit
import pyo3_gevent_example


def async_result_ready_immediately():
    ar = AsyncResult()
    ar.set(0)
    ar.wait()


def async_result_ready_greenlet():
    ar = AsyncResult()
    gevent.spawn(lambda: ar.set(0))
    ar.wait()


def gevent_spawn():
    gevent.spawn(lambda: 0)


def secs_to_well_fmt_time(secs: float):
    time = secs
    suffixes = ["s", "ms", "µs", "ns", "ps"]
    for s in suffixes:
        if time > 1:
            return f"{round(time, 2)}{s}"
        time *= 1000
    raise ValueError(f"{secs} was toooooo small")


def my_timeit(op):
    print(f"--- {op.__name__} ---")
    iters = 10_000
    total_time = timeit(op, number=iters)
    avg_time = total_time / iters
    print(f"{secs_to_well_fmt_time(avg_time)}")


tasks = [
    gevent_spawn,
    async_result_ready_immediately,
    async_result_ready_greenlet,
    pyo3_gevent_example.thread_result_ready_immediately,
    pyo3_gevent_example.thread_result_tokio_task,
    pyo3_gevent_example.thread_result_os_thread,
]

for task in tasks:
    my_timeit(task)
