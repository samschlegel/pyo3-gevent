import asyncio
import pyo3_gevent

# pyo3_gevent.init_tracing()

async def sum_as_string(a, b):
    result =  pyo3_gevent.sum_as_string(a, b)
    return result

def bench():
    tasks = []
    for i in range(100_000):
        tasks.append(asyncio.create_task(sum_as_string(i, 1)))
    asyncio.gather(*tasks)
    # for i in range(100_000):
    #     sum_as_string(i, 1)

async def main():
    import cProfile
    cProfile.run("bench()")

asyncio.run(main())

