//! The one error type shared by every networked function in this crate
//! (`actions`, `discovery`, `history`).

use std::fmt;

#[derive(Debug)]
pub enum ActionError {
    /// RPC endpoint unreachable.
    NoConnection,
    /// RPC responded but the program/transaction rejected the request.
    Rejected(String),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::NoConnection => write!(f, "no connection to RPC endpoint"),
            ActionError::Rejected(msg) => write!(f, "rejected: {msg}"),
        }
    }
}

impl std::error::Error for ActionError {}

/// Connection-level client errors (I/O, HTTP transport) become
/// `NoConnection`; everything else is `Rejected` with the error's text.
pub(crate) fn map_client_error(err: solana_client::client_error::ClientError) -> ActionError {
    match err.kind() {
        solana_client::client_error::ClientErrorKind::Io(_)
        | solana_client::client_error::ClientErrorKind::Reqwest(_) => ActionError::NoConnection,
        other => ActionError::Rejected(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_io_errors_to_no_connection() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let client_err: solana_client::client_error::ClientError = io_err.into();
        assert!(matches!(
            map_client_error(client_err),
            ActionError::NoConnection
        ));
    }

    #[test]
    fn maps_other_errors_to_rejected() {
        let client_err: solana_client::client_error::ClientError =
            solana_client::client_error::ClientErrorKind::Custom("nope".to_string()).into();
        assert!(matches!(
            map_client_error(client_err),
            ActionError::Rejected(_)
        ));
    }
}
