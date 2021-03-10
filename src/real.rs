pub use tokio::select;

pub mod io {
    use tokio::io;

    pub use io::{AsyncRead, AsyncReadExt};
    pub use io::{AsyncWrite, AsyncWriteExt};
    pub use io::{Error, ErrorKind, Result};
}

// we don't mock the types in the net module
pub mod net {
    pub use tokio::net::*;
}

pub mod task {
    pub use tokio::task::*;
}

pub mod time {
    use tokio::time;

    pub use std::time::Duration; // Re-export in tokio

    pub use time::sleep;
    pub use time::sleep_until;
    pub use time::Instant;
}

pub mod sync {
    pub mod mpsc {
        use tokio::sync::mpsc;

        pub use mpsc::channel;
        pub use mpsc::Receiver;
        pub use mpsc::Sender;

        pub use mpsc::unbounded_channel;
        pub use mpsc::UnboundedReceiver;
        pub use mpsc::UnboundedSender;

        pub mod error {
            use tokio::sync::mpsc::error;

            pub use error::RecvError;
            pub use error::SendError;
            pub use error::TrySendError;
        }
    }

    pub mod oneshot {
        use tokio::sync::oneshot;

        pub use oneshot::channel;
        pub use oneshot::Receiver;
        pub use oneshot::Sender;

        pub mod error {
            use tokio::sync::oneshot::error;

            pub use error::RecvError;
            pub use error::TryRecvError;
        }
    }

    pub mod broadcast {
        pub use tokio::sync::broadcast::*;
    }

    pub use tokio::sync::Notify;
}

// These are not mocked
pub use tokio::spawn;
