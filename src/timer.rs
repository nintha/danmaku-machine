use std::time::Duration;
use core::future::Future;
use futures::channel::oneshot;
use futures::FutureExt;

pub struct Timer {
    _close_sender: oneshot::Sender<()>,
}

impl Timer {
    pub fn new<F, Fut, T>(mut f: F, ctx: T, interval: Duration) -> Self
        where F: FnMut(T) -> Fut + Send + 'static,
              Fut: Future<Output=()> + Send + 'static,
              T: 'static + Send + Clone,
    {
        let (tx, rx) = oneshot::channel::<()>();
        async_std::task::spawn(async move {
            let mut fuse_rx = rx.fuse();
            loop {
                let sleep_future = async_std::task::sleep(interval).fuse();
                futures::pin_mut!(sleep_future);

                futures::select! {
                    _ = fuse_rx => {
                        log::info!("Timer stop");
                        break;
                    }
                    _ = sleep_future => {
                       async_std::task::spawn(f(ctx.clone()));
                    }
                }
            }
        });

        Self {
            _close_sender: tx,
        }
    }
}

