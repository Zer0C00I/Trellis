//! The one error type shared by every networked function in this crate
//! (`actions`, `discovery`, `history`).

use std::fmt;

#[derive(Debug)]
pub enum ActionError {
    /// RPC endpoint unreachable.
    NoConnection,
    /// RPC responded but the program/transaction rejected the request.
    Rejected(String),
    /// The transaction was sent but its outcome is unknown: it may or may
    /// not have landed. Do NOT retry by building a new transaction (a new
    /// blockhash makes it a different transaction and could pay twice);
    /// resolve it first with `actions::confirm_signature`.
    Unconfirmed(solana_signature::Signature),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::NoConnection => write!(f, "no connection to RPC endpoint"),
            ActionError::Rejected(msg) => write!(f, "rejected: {msg}"),
            ActionError::Unconfirmed(signature) => write!(
                f,
                "sent, but not confirmed yet (signature {signature}); check before retrying"
            ),
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

    #[test]
    fn unconfirmed_display_says_sent_unconfirmed_and_check_before_retrying() {
        let signature = solana_signature::Signature::from([7u8; 64]);
        let text = ActionError::Unconfirmed(signature).to_string();
        assert!(text.contains("sent, but not confirmed yet"), "{text}");
        assert!(text.contains(&signature.to_string()), "{text}");
        assert!(text.contains("check before retrying"), "{text}");
    }

    #[test]
    fn connection_errors_stay_no_connection_not_unconfirmed() {
        // Reqwest errors are built by the HTTP stack; Io is the constructible one.
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let client_err: solana_client::client_error::ClientError = io_err.into();
        assert!(matches!(
            map_client_error(client_err),
            ActionError::NoConnection
        ));
    }
}
