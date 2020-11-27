use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use super::{clock, Instant};

#[derive(Debug)]
pub struct Delay {
    deadline: Instant,
}

impl Delay {
    pub(crate) fn new_deadline(deadline: Instant) -> Self {
        Self { deadline }
    }

    pub(crate) fn new_delay(delay: Duration) -> Self {
        Self {
            deadline: clock::now() + delay,
        }
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    pub fn is_elapsed(&self) -> bool {
        clock::now() >= self.deadline
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_elapsed() {
            println!("Timer elasped");
            Poll::Ready(())
        } else {
            println!(
                "Timer pending (still {:?} to go)",
                self.deadline - clock::now()
            );
            Poll::Pending
        }
    }
}
