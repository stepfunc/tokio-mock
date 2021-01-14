// ---- these types are just re-exported ---

pub mod broadcast {
    pub use tokio::sync::broadcast::*;
}

pub mod watch {
    pub use tokio::sync::watch::*;
}

pub use tokio::sync::Notify;

// ---- these types are mocked ---

pub mod mpsc;
pub mod oneshot;
