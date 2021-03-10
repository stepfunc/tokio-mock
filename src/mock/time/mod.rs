mod clock;
mod instant;
mod sleep;

pub use std::time::Duration;

pub use instant::Instant;
pub use sleep::Delay;

pub fn sleep_until(deadline: Instant) -> Delay {
    Delay::new_deadline(deadline)
}

pub fn sleep(delay: Duration) -> Delay {
    Delay::new_delay(delay)
}

// Modify the time (test only)
pub fn advance(duration: Duration) {
    clock::advance(duration);
}
