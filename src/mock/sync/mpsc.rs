use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::sync::mpsc::error::{SendError, TryRecvError, TrySendError};

pub use tokio::sync::mpsc::error;

#[derive(Debug)]
struct ChannelData<T> {
    queue: VecDeque<T>,
    max_size: Option<usize>,
    num_senders: usize,
    is_active: bool,
}

impl<T> ChannelData<T> {
    fn new(max_size: Option<usize>) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
            num_senders: 1,
            is_active: true,
        }
    }

    fn try_recv(&mut self) -> Result<T, TryRecvError> {
        if let Some(msg) = self.queue.pop_front() {
            Ok(msg)
        } else if self.num_senders == 0 {
            Err(TryRecvError::Closed)
        } else {
            Err(TryRecvError::Empty)
        }
    }

    fn try_send(&mut self, value: T) -> Result<(), TrySendError<T>> {
        if !self.is_active {
            return Err(TrySendError::Closed(value));
        }

        if self
            .max_size
            .map_or(true, |max_size| self.queue.len() < max_size)
        {
            self.queue.push_back(value);
            Ok(())
        } else {
            Err(TrySendError::Full(value))
        }
    }
}

struct ReceiveFuture<T> {
    data: Arc<Mutex<ChannelData<T>>>,
}

impl<T> Future for ReceiveFuture<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.data.lock().unwrap().try_recv() {
            Ok(msg) => Poll::Ready(Some(msg)),
            Err(TryRecvError::Closed) => Poll::Ready(None),
            Err(TryRecvError::Empty) => Poll::Pending,
        }
    }
}

struct SendFuture<T> {
    data: Arc<Mutex<ChannelData<T>>>,
}

impl<T> Future for SendFuture<T> {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let data = self.data.lock().unwrap();

        if !data.is_active {
            return Poll::Ready(false);
        }

        if data
            .max_size
            .map_or(true, |max_size| data.queue.len() < max_size)
        {
            Poll::Ready(true)
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

    pub fn recv(&mut self) -> impl Future<Output = Option<T>> {
        ReceiveFuture {
            data: self.data.clone(),
        }
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        self.data.lock().unwrap().try_recv()
    }

    pub fn close(&mut self) {
        let mut data = self.data.lock().unwrap();

        data.is_active = false;
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

    pub async fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        if (SendFuture {
            data: self.data.clone(),
        })
        .await
        {
            self.data.lock().unwrap().queue.push_back(value);
            Ok(())
        } else {
            Err(SendError(value))
        }
    }

    pub fn try_send(&mut self, value: T) -> Result<(), TrySendError<T>> {
        self.data.lock().unwrap().try_send(value)
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut data = self.data.lock().unwrap();
        data.num_senders = data.num_senders.saturating_sub(1);
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        {
            let mut data = self.data.lock().unwrap();
            data.num_senders = data.num_senders.saturating_add(1);
        }

        Self {
            data: self.data.clone(),
        }
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Sender").finish()
    }
}

pub type UnboundedReceiver<T> = Receiver<T>;

pub struct UnboundedSender<T> {
    data: Arc<Mutex<ChannelData<T>>>,
}

impl<T> UnboundedSender<T> {
    fn new(data: Arc<Mutex<ChannelData<T>>>) -> Self {
        Self { data }
    }

    pub fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        let mut data = self.data.lock().unwrap();

        if data.is_active {
            data.queue.push_back(value);
            Ok(())
        } else {
            Err(SendError(value))
        }
    }

    pub fn try_send(&mut self, value: T) -> Result<(), TrySendError<T>> {
        self.data.lock().unwrap().try_send(value)
    }
}

impl<T> Drop for UnboundedSender<T> {
    fn drop(&mut self) {
        let mut data = self.data.lock().unwrap();
        data.num_senders = data.num_senders.saturating_sub(1);
    }
}

impl<T> Clone for UnboundedSender<T> {
    fn clone(&self) -> Self {
        {
            let mut data = self.data.lock().unwrap();
            data.num_senders = data.num_senders.saturating_add(1);
        }

        Self {
            data: self.data.clone(),
        }
    }
}

impl<T> fmt::Debug for UnboundedSender<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("UnboundedSender").finish()
    }
}

pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
    let data = Arc::new(Mutex::new(ChannelData::new(Some(buffer))));

    (Sender::new(data.clone()), Receiver::new(data))
}

pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let data = Arc::new(Mutex::new(ChannelData::new(None)));

    (
        UnboundedSender::new(data.clone()),
        UnboundedReceiver::new(data),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::*;

    mod bounded {
        use super::*;

        #[test]
        fn dropping_tx() {
            let (mut tx, mut rx) = channel(16);

            assert_pending!(task::spawn(async { rx.recv().await }).poll());
            assert_ready!(task::spawn(async move {
                tx.send(()).await.unwrap();
                drop(tx);
            })
            .poll());
            assert_ready_eq!(task::spawn(async { rx.recv().await }).poll(), Some(()));
            assert_ready_eq!(task::spawn(async { rx.recv().await }).poll(), None);
        }

        #[test]
        fn dropping_tx_try_recv() {
            let (mut tx, mut rx) = channel(16);

            assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
            assert_ready!(task::spawn(async move {
                tx.send(()).await.unwrap();
                drop(tx);
            })
            .poll());
            assert_eq!(rx.try_recv(), Ok(()));
            assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
        }

        #[test]
        fn dropping_rx() {
            let (mut tx1, rx) = channel(16);
            let mut tx2 = tx1.clone();

            assert_ready_ok!(task::spawn(async { tx1.send(()).await }).poll());
            assert_ready_ok!(task::spawn(async { tx2.send(()).await }).poll());

            drop(rx);

            assert_ready_err!(task::spawn(async { tx1.send(()).await }).poll());
            assert_ready_err!(task::spawn(async { tx2.send(()).await }).poll());
        }

        #[test]
        fn dropping_rx_try_send() {
            let (mut tx1, rx) = channel(16);
            let mut tx2 = tx1.clone();

            assert!(tx1.try_send(()).is_ok());
            assert!(tx2.try_send(()).is_ok());

            drop(rx);

            assert!(matches!(tx1.try_send(()), Err(TrySendError::Closed(()))));
            assert!(matches!(tx2.try_send(()), Err(TrySendError::Closed(()))));
        }

        #[test]
        fn queue_full() {
            let (mut tx, mut rx) = channel(16);

            assert_ready!(task::spawn(async {
                for _ in 0..16usize {
                    tx.send(()).await.unwrap();
                }
            })
            .poll());
            assert_pending!(task::spawn(async {
                tx.send(()).await.unwrap();
            })
            .poll());
            assert_ready!(task::spawn(async { rx.recv().await }).poll());
            assert_ready!(task::spawn(async {
                tx.send(()).await.unwrap();
            })
            .poll());
        }

        #[test]
        fn queue_full_try_send() {
            let (mut tx, mut rx) = channel(16);

            for _ in 0..16 {
                assert!(tx.try_send(()).is_ok());
            }
            assert!(matches!(tx.try_send(()), Err(TrySendError::Full(()))));
            assert!(rx.try_recv().is_ok());
            assert!(tx.try_send(()).is_ok());
        }
    }

    mod unbounded {
        use super::*;

        #[test]
        fn dropping_tx() {
            let (mut tx, mut rx) = unbounded_channel();

            assert_pending!(task::spawn(async { rx.recv().await }).poll());
            tx.send(()).unwrap();
            drop(tx);
            assert_ready_eq!(task::spawn(async { rx.recv().await }).poll(), Some(()));
            assert_ready_eq!(task::spawn(async { rx.recv().await }).poll(), None);
        }

        #[test]
        fn dropping_tx_try_recv() {
            let (mut tx, mut rx) = unbounded_channel();

            assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
            tx.send(()).unwrap();
            drop(tx);
            assert_eq!(rx.try_recv(), Ok(()));
            assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
        }

        #[test]
        fn dropping_rx() {
            let (mut tx1, rx) = unbounded_channel();
            let mut tx2 = tx1.clone();

            assert!(tx1.send(()).is_ok());
            assert!(tx2.send(()).is_ok());

            drop(rx);

            assert!(tx1.send(()).is_err());
            assert!(tx2.send(()).is_err());
        }
    }
}
