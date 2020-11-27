mod clock;
mod delay;
mod instant;

pub use std::time::Duration;

pub use delay::Delay;
pub use instant::Instant;

pub fn delay_until(deadline: Instant) -> Delay {
    Delay::new_deadline(deadline)
}

pub fn delay_for(delay: Duration) -> Delay {
    Delay::new_delay(delay)
}

// Modify the time (test only)
pub fn advance(duration: Duration) {
    clock::advance(duration);
}
