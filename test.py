from gevent import monkey
monkey.patch_all()

import gevent
from gevent.event import AsyncResult

import pyo3_gevent

def async_sum_as_string(a, b):
    # result =  pyo3_gevent.sum_as_string(a, b)
    gevent.sleep(1)
    result = 1+2
    print(result)
    return result

gevent.spawn(async_sum_as_string, 1, 2)
gevent.sleep(0.01)
print("Hello, World!")
gevent.sleep(2)



from gevent.util import print_run_info
print_run_info()

import gc
gc.collect()

print_run_info()
