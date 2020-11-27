use std::cell::RefCell;
use std::time::Duration;

thread_local!(static CLOCK: RefCell<Clock> = RefCell::new(Clock::new()));

pub(crate) fn now() -> super::Instant {
    CLOCK.with(|clock| clock.borrow().now())
}

pub(crate) fn advance(duration: Duration) {
    CLOCK.with(|clock| clock.borrow_mut().advance(duration));
}

struct Clock {
    now: std::time::Instant,
}

impl Clock {
    fn new() -> Self {
        Self {
            now: std::time::Instant::now(),
        }
    }

    pub fn now(&self) -> super::Instant {
        self.now.into()
    }

    pub fn advance(&mut self, duration: Duration) {
        self.now += duration;
    }
}
