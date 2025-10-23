use pyo3::Python;
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use pyo3_gevent::thread_result::new_thread_result;

pub fn criterion_benchmark(c: &mut Criterion) {
    Python::initialize();

    c.bench_function("new_thread_result", |b| {
        b.iter(|| black_box(new_thread_result::<(), ()>()))
    });

    c.bench_function("thread result send", |b| {
        b.iter(|| {
            let (tx, _rx) = new_thread_result::<(), ()>().unwrap();
            _ = black_box(tx.complete_ok(black_box(())));
        });
    });

    c.bench_function("thread result send and recv", |b| {
        b.iter(|| {
            let (tx, rx) = new_thread_result::<(), ()>().unwrap();
            _ = black_box(tx.complete_ok(black_box(())));

            _ = black_box(rx.wait());
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
