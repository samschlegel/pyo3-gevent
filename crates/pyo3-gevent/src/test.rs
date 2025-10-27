use std::time::Duration;

use pyo3::Python;

use crate::thread_result::new_thread_result;

#[test]
fn it_works_once() {
    Python::initialize();

    let (tx, rx) = new_thread_result::<u8, ()>().unwrap();
    tx.complete_ok(212).unwrap();
    let actual: u8 =
        Python::attach(|py| rx.wait_timeout(Duration::from_secs(1)).unwrap().extract(py)).unwrap();
    assert_eq!(212, actual);
}
