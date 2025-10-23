import gevent
import pyo3_gevent
from time import time


greenlets = [
    pyo3_gevent.sleep_thread,
    pyo3_gevent.sleep_tokio,
]

for g in greenlets:
    print(f"--- {g.__name__} ---")
    start = time()
    results = [
        x.value for x in gevent.joinall([gevent.spawn(g, i, 1000) for i in [1, 2, 3]])
    ]
    print(f"done: {time() - start}s - {results}")
    print()
