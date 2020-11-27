use std::fmt;
use std::ops;
use std::time::Duration;

use super::clock;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Instant {
    std: std::time::Instant,
}

impl Instant {
    pub fn now() -> Instant {
        clock::now()
    }

    pub fn from_std(std: std::time::Instant) -> Instant {
        Instant { std }
    }

    pub fn into_std(self) -> std::time::Instant {
        self.std
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.std.duration_since(earlier.std)
    }

    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.std.checked_duration_since(earlier.std)
    }

    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.std.saturating_duration_since(earlier.std)
    }

    pub fn elapsed(&self) -> Duration {
        Instant::now() - *self
    }

    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        self.std.checked_add(duration).map(Instant::from_std)
    }

    pub fn checked_sub(&self, duration: Duration) -> Option<Instant> {
        self.std.checked_sub(duration).map(Instant::from_std)
    }
}

impl From<std::time::Instant> for Instant {
    fn from(time: std::time::Instant) -> Instant {
        Instant::from_std(time)
    }
}

impl From<Instant> for std::time::Instant {
    fn from(time: Instant) -> std::time::Instant {
        time.into_std()
    }
}

impl ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, other: Duration) -> Instant {
        Instant::from_std(self.std + other)
    }
}

impl ops::AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl ops::Sub for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Duration {
        self.std - rhs.std
    }
}

impl ops::Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Instant {
        Instant::from_std(self.std - rhs)
    }
}

impl ops::SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl fmt::Debug for Instant {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.std.fmt(fmt)
    }
}
