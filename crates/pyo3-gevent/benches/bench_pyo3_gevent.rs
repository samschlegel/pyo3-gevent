use pyo3::{Python, types::PyAnyMethods};
use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};

use pyo3_gevent::thread_result::new_thread_result;

fn gevent_sleep_zero() {
    Python::attach(|py| {
        let gevent = py.import("gevent").unwrap();
        gevent.getattr("sleep").unwrap().call1((0,)).unwrap();
    });
}

fn bench_with_and_without_sleep(c: &mut Criterion, name: &'static str, funk: impl Fn()) {
    let mut group = c.benchmark_group(name);
    group.bench_function("no-sleep", |b| b.iter(|| funk()));
    group.bench_function("gevent.sleep(0)", |b| {
        // GC Pause
        gevent_sleep_zero();
        b.iter(|| funk())
    });
}

pub fn criterion_benchmark(c: &mut Criterion) {
    Python::initialize();

    bench_with_and_without_sleep(c, "new_thread_result", || {
        _ = black_box(new_thread_result::<(), ()>());
    });

    bench_with_and_without_sleep(c, "thread result send", || {
        let (tx, _rx) = new_thread_result::<(), ()>().unwrap();
        _ = black_box(tx.complete_ok(black_box(())));
    });

    bench_with_and_without_sleep(c, "thread result send and recv", || {
        let (tx, rx) = new_thread_result::<(), ()>().unwrap();
        _ = black_box(tx.complete_ok(black_box(())));

        _ = black_box(rx.wait());
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(20));
    targets = criterion_benchmark
);
criterion_main!(benches);
