use error::{RecvError, TryRecvError};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub mod error {
    #[derive(Debug, Eq, PartialEq)]
    pub struct RecvError;

    #[derive(Debug, Eq, PartialEq)]
    pub enum TryRecvError {
        /// The send half of the channel has not yet sent a value.
        Empty,

        /// The send half of the channel was dropped without sending a value.
        Closed,
    }
}

#[derive(Debug)]
struct ChannelData<T> {
    msg: Option<T>,
    is_recv_dropped: bool,
    is_send_dropped: bool,
}

impl<T> ChannelData<T> {
    fn new() -> Self {
        Self {
            msg: None,
            is_recv_dropped: false,
            is_send_dropped: false,
        }
    }

    fn try_recv(&mut self) -> Result<T, TryRecvError> {
        if let Some(msg) = self.msg.take() {
            Ok(msg)
        } else if self.is_send_dropped {
            Err(TryRecvError::Closed)
        } else {
            Err(TryRecvError::Empty)
        }
    }
}

struct IsClosedFuture<T> {
    data: Arc<Mutex<ChannelData<T>>>,
}

impl<T> Future for IsClosedFuture<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let data = self.data.lock().unwrap();

        if data.is_recv_dropped {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub struct Receiver<T> {
    data: Arc<Mutex<ChannelData<T>>>,
}

impl<T> Receiver<T> {
    fn new(data: Arc<Mutex<ChannelData<T>>>) -> Self {
        Self { data }
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        self.data.lock().unwrap().try_recv()
    }

    pub fn close(&mut self) {
        let mut data = self.data.lock().unwrap();

        data.is_recv_dropped = true;
    }
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, RecvError>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.try_recv() {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(TryRecvError::Closed) => Poll::Ready(Err(RecvError)),
            Err(TryRecvError::Empty) => Poll::Pending,
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.close();
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Receiver").finish()
    }
}

pub struct Sender<T> {
    data: Arc<Mutex<ChannelData<T>>>,
}

impl<T> Sender<T> {
    fn new(data: Arc<Mutex<ChannelData<T>>>) -> Self {
        Self { data }
    }

    pub fn send(self, value: T) -> Result<(), T> {
        let mut data = self.data.lock().unwrap();

        if !data.is_recv_dropped {
            data.msg.replace(value);
            Ok(())
        } else {
            Err(value)
        }
    }

    pub fn closed(&mut self) -> impl Future<Output = ()> {
        IsClosedFuture {
            data: self.data.clone(),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.data.lock().unwrap().is_recv_dropped
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut data = self.data.lock().unwrap();
        data.is_send_dropped = true;
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Sender").finish()
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let data = Arc::new(Mutex::new(ChannelData::new()));

    (Sender::new(data.clone()), Receiver::new(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::mock::test::*;

    #[test]
    fn send_recv() {
        let (tx, rx) = channel();

        let mut rx_task = spawn(async move { rx.await });

        assert_pending!(rx_task.poll());
        assert!(tx.send(42).is_ok());
        assert_ready_eq!(rx_task.poll(), Ok(42));
    }

    #[test]
    fn dropping_tx() {
        let (tx, rx) = channel::<()>();

        let mut rx_task = spawn(async { rx.await });

        assert_pending!(rx_task.poll());
        drop(tx);
        assert_ready_err!(rx_task.poll());
    }

    #[test]
    fn dropping_tx_try_recv() {
        let (tx, mut rx) = channel::<()>();

        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        drop(tx);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
    }

    #[test]
    fn dropping_rx() {
        let (tx, rx) = channel();

        assert!(!tx.is_closed());
        drop(rx);
        assert!(tx.is_closed());
        assert_eq!(tx.send(()), Err(()));
    }

    #[test]
    fn send_closed() {
        let (mut tx, rx) = channel::<()>();

        let mut closed_task = spawn(async move { tx.closed().await });

        assert_pending!(closed_task.poll());
        drop(rx);
        assert_ready!(closed_task.poll());
    }
}
