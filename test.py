import time
import timeit
from gevent import monkey
monkey.patch_all()

import gevent
from gevent.event import AsyncResult

import pyo3_gevent

pyo3_gevent.init_tracing()

def async_sum_as_string(a, b):
    result =  pyo3_gevent.async_sum_as_string(a, b)
    return result

def sum_as_string(a, b):
    result =  pyo3_gevent.sum_as_string(a, b)
    return result

def bench():
    gs = []
    for i in range(100_000):
        gs.append(gevent.spawn(sum_as_string, i, 1))
    gevent.joinall(gs)
    # for i in range(100_000):
    #     sum_as_string(i, 1)

import cProfile
cProfile.run("bench()", "restats")

import pstats
p = pstats.Stats("restats")
p.sort_stats("cumulative").print_stats(20)
# p.print_callees("pyo3_gevent.pyo3_gevent.async_sum_as_string")
