//! Kiosk-side logic for first-run family-member pairing: the credentials
//! the kiosk shows (a QR carrying a random secret, plus a 6-digit fallback
//! code), the gate that checks them, and `PairingSession`, which turns raw
//! BLE writes into an outcome. Pure logic — no Bluetooth and no chain I/O
//! here (see `ble.rs` and `family_member.rs`), so it is fully unit-tested.
//! The wire format itself lives in `family_vault_client::pairing`.

use std::time::Instant;

use family_vault_client::pairing::{
    encode_qr_payload, AuthField, FrameAssembler, FrameError, PairingRequest, AUTH_LEN, PIN_LEN,
};

/// After this many wrong attempts the kiosk rotates to fresh credentials,
/// so the guessable 6-digit code cannot be brute-forced.
pub const MAX_WRONG_ATTEMPTS: u32 = 5;

/// What the kiosk currently accepts: the 128-bit secret shown as a QR, and
/// the 6-digit fallback code shown under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingCredentials {
    secret: AuthField,
    pin: String,
    /// `pin` zero-padded to 16 bytes, exactly as a central sends it.
    pin_field: AuthField,
}

impl PairingCredentials {
    /// 16 bytes of secret followed by 4 bytes that become the PIN. (The
    /// modulo bias of `u32 % 1_000_000` is ~0.02% and irrelevant for a
    /// rotating code.)
    pub fn from_random_bytes(bytes: [u8; 20]) -> Self {
        let mut secret = [0u8; AUTH_LEN];
        secret.copy_from_slice(&bytes[..AUTH_LEN]);
        let number = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) % 1_000_000;
        let pin = format!("{number:06}");
        // `{:06}` of a value < 1_000_000 is always exactly six ASCII
        // digits, so this cannot go out of bounds.
        let mut pin_field = [0u8; AUTH_LEN];
        pin_field[..PIN_LEN].copy_from_slice(pin.as_bytes());
        Self {
            secret,
            pin,
            pin_field,
        }
    }

    pub fn secret(&self) -> &AuthField {
        &self.secret
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }

    /// The text to render as the pairing QR.
    pub fn qr_payload(&self) -> String {
        encode_qr_payload(self.secret())
    }
}

/// Fresh credentials from the OS RNG.
pub fn generate_credentials() -> std::io::Result<PairingCredentials> {
    use std::io::Read;
    let mut bytes = [0u8; 20];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(PairingCredentials::from_random_bytes(bytes))
}

/// `"482913"` -> `"482 913"`, easier to read off a screen and type.
pub fn display_pin(pin: &str) -> String {
    let (head, tail) = pin.split_at(pin.len().min(3));
    format!("{head} {tail}")
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthCheck {
    Accepted,
    Wrong { attempts_left: u32 },
    LockedOut,
}

pub struct AuthGate {
    credentials: PairingCredentials,
    wrong_attempts: u32,
}

impl AuthGate {
    pub fn new(credentials: PairingCredentials) -> Self {
        Self {
            credentials,
            wrong_attempts: 0,
        }
    }

    /// Accepts the QR secret OR the fallback code. Once locked out, every
    /// candidate (even a correct one) is refused until `rotate` installs new
    /// credentials.
    pub fn check(&mut self, auth: &AuthField) -> AuthCheck {
        if self.wrong_attempts >= MAX_WRONG_ATTEMPTS {
            return AuthCheck::LockedOut;
        }
        // Evaluate both comparisons (no short-circuit) to keep the timing
        // independent of which credential matched.
        let by_secret = constant_time_eq(auth, self.credentials.secret());
        let by_pin = constant_time_eq(auth, &self.credentials.pin_field);
        if by_secret | by_pin {
            self.wrong_attempts = 0;
            return AuthCheck::Accepted;
        }
        self.wrong_attempts += 1;
        if self.wrong_attempts >= MAX_WRONG_ATTEMPTS {
            AuthCheck::LockedOut
        } else {
            AuthCheck::Wrong {
                attempts_left: MAX_WRONG_ATTEMPTS - self.wrong_attempts,
            }
        }
    }

    pub fn rotate(&mut self, credentials: PairingCredentials) {
        self.credentials = credentials;
        self.wrong_attempts = 0;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WriteOutcome {
    /// A partial frame was buffered; wait for more writes.
    NeedMore,
    /// A complete frame carrying valid credentials.
    Accepted(PairingRequest),
    WrongAuth { attempts_left: u32 },
    /// Too many wrong attempts. `new_credentials` are the freshly generated
    /// ones to show, or `None` if generating them failed (the gate then
    /// stays locked, which is the safe direction).
    LockedOut {
        new_credentials: Option<PairingCredentials>,
    },
    Malformed(FrameError),
}

pub struct PairingSession {
    assembler: FrameAssembler,
    gate: AuthGate,
}

impl PairingSession {
    pub fn new(credentials: PairingCredentials) -> Self {
        Self {
            assembler: FrameAssembler::default(),
            gate: AuthGate::new(credentials),
        }
    }

    /// Handles one BLE write. `next_credentials` is only called when the
    /// gate locks out, to produce the replacement credentials.
    pub fn on_write(
        &mut self,
        chunk: &[u8],
        now: Instant,
        next_credentials: impl FnOnce() -> Option<PairingCredentials>,
    ) -> WriteOutcome {
        match self.assembler.push(chunk, now) {
            Ok(None) => WriteOutcome::NeedMore,
            Err(e) => WriteOutcome::Malformed(e),
            Ok(Some(request)) => match self.gate.check(&request.auth) {
                AuthCheck::Accepted => WriteOutcome::Accepted(request),
                AuthCheck::Wrong { attempts_left } => WriteOutcome::WrongAuth { attempts_left },
                AuthCheck::LockedOut => {
                    let new_credentials = next_credentials();
                    if let Some(credentials) = &new_credentials {
                        self.gate.rotate(credentials.clone());
                    }
                    WriteOutcome::LockedOut { new_credentials }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use family_vault_client::pairing::{auth_from_pin, encode_frame, PairingRequest};
    use solana_pubkey::Pubkey;

    /// secret = [1; 16]; pin = u32::from_le_bytes([1;4]) % 1_000_000 = 843009
    fn creds() -> PairingCredentials {
        PairingCredentials::from_random_bytes([1u8; 20])
    }

    fn other_creds() -> PairingCredentials {
        PairingCredentials::from_random_bytes([2u8; 20])
    }

    fn frame_with_auth(auth: AuthField) -> Vec<u8> {
        encode_frame(&request_with_auth(auth)).unwrap()
    }

    fn request_with_auth(auth: AuthField) -> PairingRequest {
        PairingRequest {
            auth,
            pubkey: Pubkey::new_from_array([7u8; 32]),
            name: "Внук Саша".to_string(),
        }
    }

    fn wrong_auth() -> AuthField {
        [0xEE; AUTH_LEN]
    }

    // --- credentials ----------------------------------------------------

    #[test]
    fn from_random_bytes_derives_a_zero_padded_six_digit_pin() {
        let mut bytes = [9u8; 20];
        bytes[16..].copy_from_slice(&42u32.to_le_bytes());
        let c = PairingCredentials::from_random_bytes(bytes);
        assert_eq!(c.pin(), "000042");
        assert_eq!(c.secret(), &[9u8; 16]);
    }

    #[test]
    fn the_pin_field_matches_the_shared_encoding_of_the_pin() {
        let c = creds();
        assert_eq!(c.pin_field, auth_from_pin(c.pin()).unwrap());
    }

    #[test]
    fn qr_payload_carries_the_secret() {
        let c = creds();
        assert_eq!(
            family_vault_client::pairing::decode_qr_payload(&c.qr_payload()),
            Some(*c.secret())
        );
    }

    #[test]
    fn generated_credentials_are_well_formed_and_differ_between_calls() {
        let a = generate_credentials().unwrap();
        let b = generate_credentials().unwrap();
        assert_eq!(a.pin().len(), 6);
        assert!(a.pin().bytes().all(|x| x.is_ascii_digit()));
        assert_ne!(a.secret(), b.secret());
    }

    #[test]
    fn display_pin_groups_digits_in_threes() {
        assert_eq!(display_pin("482913"), "482 913");
    }

    // --- gate -----------------------------------------------------------

    #[test]
    fn gate_accepts_the_qr_secret() {
        let mut gate = AuthGate::new(creds());
        assert_eq!(gate.check(creds().secret()), AuthCheck::Accepted);
    }

    #[test]
    fn gate_accepts_the_fallback_code() {
        let mut gate = AuthGate::new(creds());
        let field = auth_from_pin(creds().pin()).unwrap();
        assert_eq!(gate.check(&field), AuthCheck::Accepted);
    }

    #[test]
    fn gate_rejects_an_all_zero_auth_field() {
        // Guards against ever treating an unset/zero field as a match.
        let mut gate = AuthGate::new(creds());
        assert_eq!(
            gate.check(&[0u8; AUTH_LEN]),
            AuthCheck::Wrong { attempts_left: 4 }
        );
    }

    #[test]
    fn gate_counts_down_wrong_attempts_then_locks_out() {
        let mut gate = AuthGate::new(creds());
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::Wrong { attempts_left: 4 });
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::Wrong { attempts_left: 3 });
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::Wrong { attempts_left: 2 });
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::Wrong { attempts_left: 1 });
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::LockedOut);
    }

    #[test]
    fn a_locked_gate_rejects_even_correct_credentials_until_rotated() {
        let mut gate = AuthGate::new(creds());
        for _ in 0..MAX_WRONG_ATTEMPTS {
            gate.check(&wrong_auth());
        }
        assert_eq!(gate.check(creds().secret()), AuthCheck::LockedOut);

        gate.rotate(other_creds());
        // Old credentials are dead, new ones work.
        assert_eq!(
            gate.check(creds().secret()),
            AuthCheck::Wrong { attempts_left: 4 }
        );
        assert_eq!(
            gate.check(&auth_from_pin(creds().pin()).unwrap()),
            AuthCheck::Wrong { attempts_left: 3 }
        );
        assert_eq!(gate.check(other_creds().secret()), AuthCheck::Accepted);
    }

    // --- session --------------------------------------------------------

    #[test]
    fn session_accepts_the_qr_secret_in_one_write() {
        let mut session = PairingSession::new(creds());
        let out = session.on_write(&frame_with_auth(*creds().secret()), Instant::now(), || None);
        assert_eq!(
            out,
            WriteOutcome::Accepted(request_with_auth(*creds().secret()))
        );
    }

    #[test]
    fn session_accepts_the_fallback_code() {
        let mut session = PairingSession::new(creds());
        let field = auth_from_pin(creds().pin()).unwrap();
        let out = session.on_write(&frame_with_auth(field), Instant::now(), || None);
        assert_eq!(out, WriteOutcome::Accepted(request_with_auth(field)));
    }

    #[test]
    fn session_asks_for_more_until_the_frame_is_complete() {
        let mut session = PairingSession::new(creds());
        let now = Instant::now();
        let frame = frame_with_auth(*creds().secret());
        assert_eq!(session.on_write(&frame[..20], now, || None), WriteOutcome::NeedMore);
        assert_eq!(
            session.on_write(&frame[20..], now, || None),
            WriteOutcome::Accepted(request_with_auth(*creds().secret()))
        );
    }

    #[test]
    fn session_reports_wrong_credentials() {
        let mut session = PairingSession::new(creds());
        let out = session.on_write(&frame_with_auth(wrong_auth()), Instant::now(), || None);
        assert_eq!(out, WriteOutcome::WrongAuth { attempts_left: 4 });
    }

    #[test]
    fn session_reports_malformed_frames() {
        let mut session = PairingSession::new(creds());
        let mut frame = frame_with_auth(*creds().secret());
        frame.push(0); // trailing byte
        let out = session.on_write(&frame, Instant::now(), || None);
        assert_eq!(out, WriteOutcome::Malformed(FrameError::TooLong));
    }

    #[test]
    fn session_rotates_credentials_after_too_many_wrong_attempts() {
        let mut session = PairingSession::new(creds());
        let now = Instant::now();
        for _ in 0..4 {
            session.on_write(&frame_with_auth(wrong_auth()), now, || None);
        }
        let out = session.on_write(&frame_with_auth(wrong_auth()), now, || Some(other_creds()));
        assert_eq!(
            out,
            WriteOutcome::LockedOut {
                new_credentials: Some(other_creds())
            }
        );

        // The old QR secret and old code are dead, the new secret works.
        assert_eq!(
            session.on_write(&frame_with_auth(*creds().secret()), now, || None),
            WriteOutcome::WrongAuth { attempts_left: 4 }
        );
        assert_eq!(
            session.on_write(&frame_with_auth(*other_creds().secret()), now, || None),
            WriteOutcome::Accepted(request_with_auth(*other_creds().secret()))
        );
    }

    #[test]
    fn session_stays_locked_if_new_credentials_cannot_be_generated() {
        let mut session = PairingSession::new(creds());
        let now = Instant::now();
        for _ in 0..5 {
            session.on_write(&frame_with_auth(wrong_auth()), now, || None);
        }
        // Even the correct secret must not get in while locked and un-rotated.
        let out = session.on_write(&frame_with_auth(*creds().secret()), now, || None);
        assert_eq!(out, WriteOutcome::LockedOut { new_credentials: None });
    }
}
