pub mod io;
pub mod net;
pub mod sync;
pub mod test;
pub mod time;

// Re-exports from tokio
pub use tokio::select;

// This is not mocked, but `test::spawn` can be used
// to poll manually.
pub use tokio::spawn;

#[cfg(test)]
mod tests {
    use crate::mock::test::*;
    use std::future::Future;
    use std::time::Duration;

    use super::*;

    #[test]
    fn select_message_before_timeout() {
        let (mut tx, rx) = sync::mpsc::channel::<u32>(16);

        let mut select_task = select_task(rx);

        assert_pending!(select_task.poll());
        assert_ready_ok!(crate::mock::test::spawn(async { tx.send(42).await }).poll());
        assert_ready_eq!(select_task.poll(), Some(42));
    }

    #[test]
    fn select_timeout_before_message() {
        let (_tx, rx) = sync::mpsc::channel::<u32>(16);

        let mut select_task = select_task(rx);

        assert_pending!(select_task.poll());
        time::advance(Duration::from_secs(1));
        assert_ready_eq!(select_task.poll(), None);
    }

    fn select_task(mut rx: sync::mpsc::Receiver<u32>) -> Spawn<impl Future<Output = Option<u32>>> {
        crate::mock::test::spawn(async move {
            tokio::select! {
                _ = time::delay_for(Duration::from_secs(1)) => {
                    None
                }
                value = rx.recv() => {
                    Some(value.unwrap())
                }
            }
        })
    }
}
