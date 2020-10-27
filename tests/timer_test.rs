use danmaku_machine::timer::Timer;
use std::time::Duration;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use danmaku_machine::common::init_logger;

#[test]
fn interval_print() {
    init_logger();
    let (tx, rx) = std::sync::mpsc::channel();

    let counter = Arc::new(AtomicU32::new(0));
    let _timer = Timer::new(|(tx, counter)| async move {
        let count = counter.fetch_add(1, Ordering::SeqCst);
        log::info!("count={}", count);
        tx.send(count).unwrap();
    }, (tx.clone(), counter.clone()), Duration::from_millis(100), );

    for _ in 0..4 {
        log::info!("{:?}", rx.recv());
    }

    assert_eq!(4, counter.load(Ordering::SeqCst));
}
