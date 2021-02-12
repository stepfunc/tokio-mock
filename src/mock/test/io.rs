use super::super::io::{AsyncRead, AsyncWrite, Error, ErrorKind};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::ReadBuf;

#[derive(Debug)]
enum ChannelState {
    Open(VecDeque<Box<[u8]>>),
    Closed,
    Error(ErrorKind),
}

impl ChannelState {
    fn new() -> Self {
        Self::Open(VecDeque::new())
    }

    fn push(&mut self, data: &[u8]) {
        match self {
            Self::Open(buffer) => {
                buffer.push_back(Vec::from(data).into_boxed_slice());
            }
            _ => {
                let mut buffer = VecDeque::new();
                buffer.push_back(Vec::from(data).into_boxed_slice());
                *self = Self::Open(buffer);
            }
        }
    }

    fn close(&mut self) {
        *self = Self::Closed;
    }

    fn error(&mut self, err: ErrorKind) {
        *self = Self::Error(err);
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Open(buffer) => buffer.is_empty(),
            _ => true,
        }
    }
}

#[derive(Debug)]
struct Shared {
    write_pending: bool,
    write_channel: ChannelState,
    read_channel: ChannelState,
}

impl Shared {
    fn new() -> Self {
        Self {
            write_pending: false,
            write_channel: ChannelState::new(),
            read_channel: ChannelState::new(),
        }
    }
}

pub struct Handle(Arc<Mutex<Shared>>);

impl Handle {
    pub fn read(&mut self, data: &[u8]) {
        self.0.lock().unwrap().read_channel.push(data);
    }

    pub fn all_read(&self) -> bool {
        self.0.lock().unwrap().read_channel.is_empty()
    }

    pub fn close_read(&mut self) {
        self.0.lock().unwrap().read_channel.close();
    }

    pub fn read_error(&mut self, err: ErrorKind) {
        self.0.lock().unwrap().read_channel.error(err);
    }

    pub fn all_written(&self) -> bool {
        self.0.lock().unwrap().write_channel.is_empty()
    }

    pub fn pending_write(&self) -> bool {
        self.0.lock().unwrap().write_pending
    }

    pub fn write(&mut self, data: &[u8]) {
        self.0.lock().unwrap().write_channel.push(data);
    }

    pub fn close_write(&mut self) {
        self.0.lock().unwrap().write_channel.close();
    }

    pub fn write_error(&mut self, err: ErrorKind) {
        self.0.lock().unwrap().write_channel.error(err);
    }

    pub fn all_done(&self) -> bool {
        self.all_read() && self.all_written()
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if let Ok(mut shared) = self.0.lock() {
            shared.read_channel.close();
            shared.write_channel.close();
        }
    }
}

#[derive(Debug)]
pub struct MockIO(Arc<Mutex<Shared>>);

impl AsyncRead for MockIO {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut shared = self.0.lock().unwrap();

        match &mut shared.read_channel {
            ChannelState::Open(data) => {
                match data.pop_front() {
                    None => Poll::Pending,
                    Some(bytes) => {
                        if bytes.len() > buf.remaining() {
                            panic!("insufficient write space (available == {}) for queued read (len = {})", buf.remaining(), bytes.len());
                        }
                        buf.put_slice(&bytes);
                        Poll::Ready(Ok(()))
                    }
                }
            }
            ChannelState::Closed => Poll::Ready(Ok(())),
            ChannelState::Error(err) => Poll::Ready(Err(Error::new(*err, "test error"))),
        }
    }
}

impl AsyncWrite for MockIO {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let mut shared = self.0.lock().unwrap();

        match &mut shared.write_channel {
            ChannelState::Open(data) => match data.pop_front() {
                None => {
                    shared.write_pending = true;
                    Poll::Pending
                }
                Some(expected) => {
                    if buf != expected.as_ref() {
                        panic!(
                            r#"unexpected write:
 expected: {:02X?},
 received: {:02X?}"#,
                            expected, buf
                        );
                    }
                    shared.write_pending = false;
                    Poll::Ready(Ok(expected.len()))
                }
            },
            ChannelState::Closed => Poll::Ready(Ok(0)),
            ChannelState::Error(err) => Poll::Ready(Err(Error::new(*err, "test error"))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

pub fn mock() -> (MockIO, Handle) {
    let shared = Arc::new(Mutex::new(Shared::new()));

    (MockIO(shared.clone()), Handle(shared))
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

    #[test]
    fn dropping_handle_closes_both_channels() {
        let (mut io, handle) = mock();

        let mut read_task = spawn(async {
            let mut buf = [0, 20];
            io.read(&mut buf).await.unwrap()
        });

        assert_pending!(read_task.poll());
        drop(handle);
        assert_ready_eq!(read_task.poll(), 0);
    }
}
