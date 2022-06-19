use super::super::io::{AsyncRead, AsyncWrite, Error, ErrorKind};

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::ReadBuf;

#[derive(Debug)]
enum Action {
    Read(Vec<u8>),
    Write(Vec<u8>),
    ReadError(ErrorKind),
    WriteError(ErrorKind),
}

#[derive(Debug)]
struct Inner {
    actions: VecDeque<Action>,
}

impl Inner {
    fn new() -> Self {
        Self {
            actions: VecDeque::new(),
        }
    }
}

pub struct Handle {
    inner: Arc<Mutex<Inner>>,
}

impl Handle {
    pub fn read(&mut self, data: &[u8]) {
        self.inner
            .lock()
            .unwrap()
            .actions
            .push_back(Action::Read(Vec::from(data)));
    }

    pub fn read_error(&mut self, err: ErrorKind) {
        self.inner
            .lock()
            .unwrap()
            .actions
            .push_back(Action::ReadError(err));
    }

    pub fn write(&mut self, data: &[u8]) {
        self.inner
            .lock()
            .unwrap()
            .actions
            .push_back(Action::Write(Vec::from(data)));
    }

    pub fn write_error(&mut self, err: ErrorKind) {
        self.inner
            .lock()
            .unwrap()
            .actions
            .push_back(Action::WriteError(err));
    }
}

#[derive(Debug)]
pub struct MockIo {
    inner: Arc<Mutex<Inner>>,
}

impl Drop for MockIo {
    fn drop(&mut self) {
        let count = self.inner.lock().unwrap().actions.len();
        if count > 0 && !std::thread::panicking() {
            panic!("I/O script contained {} actions on drop", count)
        }
    }
}

impl AsyncRead for MockIo {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let (pop, result) = match self.inner.lock().unwrap().actions.front() {
            Some(Action::Read(bytes)) => {
                if bytes.len() > buf.remaining() {
                    panic!(
                        "insufficient write space (available == {}) for queued read (len = {})",
                        buf.remaining(),
                        bytes.len()
                    );
                }
                buf.put_slice(bytes);
                (true, Poll::Ready(Ok(())))
            }
            Some(Action::ReadError(kind)) => {
                (true, Poll::Ready(Err(Error::new(*kind, "test error"))))
            }
            _ => (false, Poll::Pending),
        };

        if pop {
            self.inner.lock().unwrap().actions.pop_front().unwrap();
        }

        result
    }
}

impl AsyncWrite for MockIo {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let (pop, result) = match self.inner.lock().unwrap().actions.front() {
            Some(Action::Write(bytes)) => {
                if buf != bytes.as_slice() {
                    panic!(
                        r#"unexpected write:
 expected: {:02X?},
 received: {:02X?}"#,
                        bytes, buf
                    );
                }
                (true, Poll::Ready(Ok(bytes.len())))
            }
            Some(Action::WriteError(kind)) => {
                (true, Poll::Ready(Err(Error::new(*kind, "test error"))))
            }
            _ => (false, Poll::Pending),
        };

        if pop {
            self.inner.lock().unwrap().actions.pop_front().unwrap();
        }
        result
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

pub fn mock() -> (MockIo, Handle) {
    let inner = Arc::new(Mutex::new(Inner::new()));

    (
        MockIo {
            inner: inner.clone(),
        },
        Handle { inner },
    )
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::mock::io::AsyncReadExt;

    #[test]
    fn io_read() {
        let (mut io, mut handle) = mock();

        let mut read_task = spawn(async {
            let mut buf = [0, 20];
            io.read(&mut buf).await.unwrap()
        });

        assert_pending!(read_task.poll());
        handle.read(&[42]);
        assert_ready_eq!(read_task.poll(), 1);
        drop(read_task);
        assert_pending!(spawn(async {
            let mut buf = [0, 20];
            io.read(&mut buf).await.unwrap()
        })
        .poll());
    }
}
