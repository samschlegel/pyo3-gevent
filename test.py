import time
import timeit
from gevent import monkey
monkey.patch_all()

import gevent
from gevent.event import AsyncResult

import pyo3_gevent

def async_sum_as_string(a, b):
    result =  pyo3_gevent.sum_as_string(a, b)
    return result

print(async_sum_as_string(1,2))
print(async_sum_as_string(2,3))
