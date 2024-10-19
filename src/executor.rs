use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Poll, Wake, Waker, Context};
use std::thread::{spawn, sleep};
use std::pin::{pin, Pin};
use std::time::Duration;
use std::future::Future;
use std::sync::Arc;

use async_channel::Receiver;

pub struct Task {
    inner: Pin<Box<dyn Future<Output = ()> + Send>>
}

impl<F: Future<Output = ()> + Send + 'static> From<F> for Task {
    fn from(fut: F) -> Self {
        Self { inner: Box::pin(fut) }
    }
}

#[derive(Debug, Default)]
struct Running {
    inner: AtomicBool,
}

impl Running {
    fn reset(&self) {
        self.inner.store(false, Ordering::SeqCst);
    }

    fn read(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }
}

impl Wake for Running {
    fn wake(self: Arc<Self>) {
        self.inner.store(true, Ordering::SeqCst);
    }
}

pub fn runner(rx_tasks: Receiver<Task>) {
    let running = Arc::new(Running::default());
    let waker = Waker::from(running.clone());

    let mut receiver = pin!(rx_tasks.recv());
    let mut can_receive_tasks = true;
    let mut has_tasks = false;
    let mut tasks = Vec::new();
    running.wake_by_ref();

    while can_receive_tasks || has_tasks {
        if !running.read() {
            sleep(Duration::from_millis(5));
            continue;
        }

        running.reset();
        let mut context = Context::from_waker(&waker);

        can_receive_tasks = loop {
            match Future::poll(receiver.as_mut(), &mut context) {
                Poll::Ready(Ok(new_task)) => tasks.push(new_task),
                Poll::Ready(Err(_)) => break false,
                Poll::Pending => break true,
            }
        };

        let mut i = 0;
        while i < tasks.len() {
            let fut = tasks[i].inner.as_mut();
            match Future::poll(fut, &mut context) {
                Poll::Ready(()) => _ = tasks.swap_remove(i),
                Poll::Pending => i += 1,
            }
        }

        has_tasks = !tasks.is_empty();
    }
}

pub fn spawn_runner(rx_tasks: &Receiver<Task>) {
    let rx_tasks = rx_tasks.clone();
    spawn(|| runner(rx_tasks));
}
