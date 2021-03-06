pub mod io;
mod macros;

use std::ptr::null;
use std::task::{Context, RawWaker, Waker};

pub use crate::assert_err;
pub use crate::assert_ok;
pub use crate::assert_pending;
pub use crate::assert_ready;
pub use crate::assert_ready_eq;
pub use crate::assert_ready_err;
pub use crate::assert_ready_ok;

pub struct Spawn<T>
where
    T: std::future::Future,
{
    future: std::pin::Pin<Box<T>>,
}

impl<T> Spawn<T>
where
    T: std::future::Future,
{
    pub fn poll(&mut self) -> std::task::Poll<T::Output> {
        let waker = unsafe { Waker::from_raw(RawWaker::new(null(), &details::NULL_WAKER_VTABLE)) };
        let mut context = Context::from_waker(&waker);
        self.future.as_mut().poll(&mut context)
    }
}

pub fn spawn<T>(f: T) -> Spawn<T>
where
    T: std::future::Future,
{
    Spawn {
        future: Box::pin(f),
    }
}

pub(crate) mod details {
    use std::ptr::null;
    use std::task::{RawWaker, RawWakerVTable};

    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(null(), &NULL_WAKER_VTABLE)
    }
    fn wake(_: *const ()) {}
    fn wake_by_ref(_: *const ()) {}
    fn drop(_: *const ()) {}

    pub(crate) const NULL_WAKER_VTABLE: RawWakerVTable =
        RawWakerVTable::new(clone, wake, wake_by_ref, drop);
}
