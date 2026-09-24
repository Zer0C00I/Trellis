//! Kiosk-side logic for first-run family-member pairing: the credentials
//! the kiosk shows (a QR carrying a random secret, plus a 6-digit fallback
//! code), the gate that checks them, and `PairingSession`, which turns raw
//! BLE writes into an outcome. Pure logic — no Bluetooth and no chain I/O
//! here (see `ble.rs` and `family_member.rs`), so it is fully unit-tested.
//! The wire format itself lives in `family_vault_client::pairing`.

use std::fmt;
use std::time::{Duration, Instant};

use family_vault_client::pairing::{
    encode_qr_payload, AuthField, FrameAssembler, FrameError, PairingRequest, AUTH_LEN, PIN_LEN,
};

/// After this many wrong attempts the kiosk rotates to fresh credentials
/// and starts a cooldown.
pub const MAX_WRONG_ATTEMPTS: u32 = 5;

/// Cooldown after the first lockout; it doubles with each further one.
pub const COOLDOWN_BASE: Duration = Duration::from_secs(30);

/// The cooldown never grows beyond this. There is no permanent lockout.
pub const COOLDOWN_MAX: Duration = Duration::from_secs(300);

/// Cooldown after the `n`th lockout (1 for the first):
/// `min(30 * 2^(n-1), 300)` seconds. Never overflows.
pub fn cooldown_for_lockout(n: u32) -> Duration {
    let doublings = n.saturating_sub(1).min(16);
    COOLDOWN_BASE
        .saturating_mul(1 << doublings)
        .min(COOLDOWN_MAX)
}

/// `30 s` -> `"30 сек."`, `60 s` -> `"1 мин."`; partial minutes round up.
/// Abbreviations avoid Russian plural forms.
pub fn display_wait(wait: Duration) -> String {
    let secs = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    if secs < 60 {
        format!("{secs} сек.")
    } else {
        format!("{} мин.", secs.div_ceil(60))
    }
}

/// The setup-screen instruction above the pairing QR. During a cooldown it
/// tells the family member to wait; without one it is the normal text.
pub fn pairing_instruction(wait: Option<Duration>) -> String {
    match wait {
        None => "Отсканируйте этот код в приложении семьи или введите цифры:".to_string(),
        Some(wait) => format!(
            "Подождите {}, затем отсканируйте этот код в приложении семьи или введите цифры:",
            display_wait(wait)
        ),
    }
}

/// Monotonic counter identifying the current state of the pairing screen.
/// Each redraw or teardown takes a new generation; a delayed action (the
/// end-of-cooldown text restore) remembers the generation it was scheduled
/// under and does nothing if the screen has moved on since.
pub struct ViewGeneration(std::sync::atomic::AtomicU64);

impl ViewGeneration {
    pub const fn new() -> Self {
        Self(std::sync::atomic::AtomicU64::new(0))
    }

    /// Starts a new generation and returns it.
    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }

    pub fn is_current(&self, generation: u64) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst) == generation
    }
}

/// What the kiosk currently accepts: the 128-bit secret shown as a QR, and
/// the 6-digit fallback code shown under it.
#[derive(Clone, PartialEq, Eq)]
pub struct PairingCredentials {
    secret: AuthField,
    pin: String,
    /// `pin` zero-padded to 16 bytes, exactly as a central sends it.
    pin_field: AuthField,
}

// Hand-written so the secret and the code can never reach a log via `{:?}`.
impl fmt::Debug for PairingCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PairingCredentials")
            .field("secret", &format_args!("<redacted>"))
            .field("pin", &format_args!("<redacted>"))
            .field("pin_field", &format_args!("<redacted>"))
            .finish()
    }
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
    Wrong {
        attempts_left: u32,
    },
    LockedOut,
    /// A cooldown is running; the credentials were not evaluated.
    CoolingDown {
        remaining: Duration,
    },
}

pub struct AuthGate {
    credentials: PairingCredentials,
    wrong_attempts: u32,
    /// Lockouts (rotations) so far; sets the cooldown length.
    lockouts: u32,
    cooldown_until: Option<Instant>,
}

impl AuthGate {
    pub fn new(credentials: PairingCredentials) -> Self {
        Self {
            credentials,
            wrong_attempts: 0,
            lockouts: 0,
            cooldown_until: None,
        }
    }

    /// Accepts the QR secret OR the fallback code. While a cooldown runs,
    /// every candidate (even a correct one) is refused without being
    /// evaluated or counted. Once locked out, every candidate is refused
    /// until `rotate` installs new credentials.
    pub fn check(&mut self, auth: &AuthField, now: Instant) -> AuthCheck {
        if let Some(until) = self.cooldown_until {
            if now < until {
                return AuthCheck::CoolingDown {
                    remaining: until - now,
                };
            }
            self.cooldown_until = None;
        }
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

    /// Installs new credentials after a lockout and starts the cooldown
    /// that applies to them; returns its length.
    pub fn rotate(&mut self, credentials: PairingCredentials, now: Instant) -> Duration {
        self.credentials = credentials;
        self.wrong_attempts = 0;
        self.lockouts = self.lockouts.saturating_add(1);
        let cooldown = cooldown_for_lockout(self.lockouts);
        self.cooldown_until = Some(now + cooldown);
        cooldown
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WriteOutcome {
    /// A partial frame was buffered; wait for more writes.
    NeedMore,
    /// A complete frame carrying valid credentials.
    Accepted(PairingRequest),
    WrongAuth {
        attempts_left: u32,
    },
    /// A cooldown is running: the frame was refused unevaluated.
    CoolingDown {
        remaining: Duration,
    },
    /// Too many wrong attempts. `new_credentials` are the freshly generated
    /// ones to show, or `None` if generating them failed (the gate then
    /// stays locked, which is the safe direction). `cooldown` is the wait
    /// that applies to the new credentials (zero when there are none).
    LockedOut {
        new_credentials: Option<PairingCredentials>,
        cooldown: Duration,
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
            Ok(Some(request)) => match self.gate.check(&request.auth, now) {
                AuthCheck::Accepted => WriteOutcome::Accepted(request),
                AuthCheck::Wrong { attempts_left } => WriteOutcome::WrongAuth { attempts_left },
                AuthCheck::CoolingDown { remaining } => WriteOutcome::CoolingDown { remaining },
                AuthCheck::LockedOut => {
                    let new_credentials = next_credentials();
                    let cooldown = match &new_credentials {
                        Some(credentials) => self.gate.rotate(credentials.clone(), now),
                        None => Duration::ZERO,
                    };
                    WriteOutcome::LockedOut {
                        new_credentials,
                        cooldown,
                    }
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
    use std::time::Duration;

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

    // --- cooldown schedule ------------------------------------------------

    #[test]
    fn cooldown_schedule_doubles_up_to_the_maximum() {
        let secs: Vec<u64> = (1..=6).map(|n| cooldown_for_lockout(n).as_secs()).collect();
        assert_eq!(secs, vec![30, 60, 120, 240, 300, 300]);
        assert_eq!(COOLDOWN_BASE, Duration::from_secs(30));
        assert_eq!(COOLDOWN_MAX, Duration::from_secs(300));
    }

    #[test]
    fn cooldown_schedule_does_not_overflow() {
        assert_eq!(cooldown_for_lockout(u32::MAX), COOLDOWN_MAX);
        assert_eq!(cooldown_for_lockout(64), COOLDOWN_MAX);
        assert_eq!(cooldown_for_lockout(0), COOLDOWN_BASE);
    }

    #[test]
    fn display_wait_uses_seconds_below_a_minute_and_minutes_above() {
        assert_eq!(display_wait(Duration::from_secs(30)), "30 сек.");
        assert_eq!(display_wait(Duration::from_secs(59)), "59 сек.");
        assert_eq!(display_wait(Duration::from_secs(60)), "1 мин.");
        assert_eq!(display_wait(Duration::from_secs(90)), "2 мин.");
        assert_eq!(display_wait(Duration::from_secs(120)), "2 мин.");
        assert_eq!(display_wait(Duration::from_secs(300)), "5 мин.");
    }

    #[test]
    fn pairing_instruction_is_the_normal_text_without_a_wait() {
        assert_eq!(
            pairing_instruction(None),
            "Отсканируйте этот код в приложении семьи или введите цифры:"
        );
    }

    #[test]
    fn pairing_instruction_prefixes_the_wait_during_a_cooldown() {
        assert_eq!(
            pairing_instruction(Some(Duration::from_secs(30))),
            "Подождите 30 сек., затем отсканируйте этот код в приложении семьи или введите цифры:"
        );
    }

    #[test]
    fn view_generation_next_increments() {
        let g = ViewGeneration::new();
        assert_eq!(g.next(), 1);
        assert_eq!(g.next(), 2);
    }

    #[test]
    fn view_generation_old_is_not_current_after_next() {
        let g = ViewGeneration::new();
        let old = g.next();
        let newest = g.next();
        assert!(!g.is_current(old));
        assert!(g.is_current(newest));
    }

    // --- gate -----------------------------------------------------------

    #[test]
    fn gate_accepts_the_qr_secret() {
        let mut gate = AuthGate::new(creds());
        assert_eq!(
            gate.check(creds().secret(), Instant::now()),
            AuthCheck::Accepted
        );
    }

    #[test]
    fn gate_accepts_the_fallback_code() {
        let mut gate = AuthGate::new(creds());
        let field = auth_from_pin(creds().pin()).unwrap();
        assert_eq!(gate.check(&field, Instant::now()), AuthCheck::Accepted);
    }

    #[test]
    fn gate_rejects_an_all_zero_auth_field() {
        // Guards against ever treating an unset/zero field as a match.
        let mut gate = AuthGate::new(creds());
        assert_eq!(
            gate.check(&[0u8; AUTH_LEN], Instant::now()),
            AuthCheck::Wrong { attempts_left: 4 }
        );
    }

    #[test]
    fn gate_counts_down_wrong_attempts_then_locks_out() {
        let mut gate = AuthGate::new(creds());
        let now = Instant::now();
        for left in [4, 3, 2, 1] {
            assert_eq!(
                gate.check(&wrong_auth(), now),
                AuthCheck::Wrong {
                    attempts_left: left
                }
            );
        }
        assert_eq!(gate.check(&wrong_auth(), now), AuthCheck::LockedOut);
    }

    #[test]
    fn a_locked_gate_rejects_even_correct_credentials_until_rotated() {
        let mut gate = AuthGate::new(creds());
        let now = Instant::now();
        for _ in 0..MAX_WRONG_ATTEMPTS {
            gate.check(&wrong_auth(), now);
        }
        assert_eq!(gate.check(creds().secret(), now), AuthCheck::LockedOut);

        assert_eq!(gate.rotate(other_creds(), now), COOLDOWN_BASE);
        let later = now + COOLDOWN_BASE;
        // Old credentials are dead, new ones work.
        assert_eq!(
            gate.check(creds().secret(), later),
            AuthCheck::Wrong { attempts_left: 4 }
        );
        assert_eq!(
            gate.check(&auth_from_pin(creds().pin()).unwrap(), later),
            AuthCheck::Wrong { attempts_left: 3 }
        );
        assert_eq!(
            gate.check(other_creds().secret(), later),
            AuthCheck::Accepted
        );
    }

    #[test]
    fn an_accepted_check_resets_the_window_counter() {
        let mut gate = AuthGate::new(creds());
        let now = Instant::now();
        gate.check(&wrong_auth(), now);
        gate.check(&wrong_auth(), now);
        assert_eq!(gate.check(creds().secret(), now), AuthCheck::Accepted);
        assert_eq!(
            gate.check(&wrong_auth(), now),
            AuthCheck::Wrong { attempts_left: 4 }
        );
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
        assert_eq!(
            session.on_write(&frame[..20], now, || None),
            WriteOutcome::NeedMore
        );
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

    /// Sends `n` wrong frames at `now`; every rotation hands out `next`.
    fn send_wrong(
        session: &mut PairingSession,
        n: u32,
        now: Instant,
        next: PairingCredentials,
    ) -> WriteOutcome {
        let mut out = None;
        for _ in 0..n {
            out =
                Some(session.on_write(&frame_with_auth(wrong_auth()), now, || Some(next.clone())));
        }
        out.unwrap()
    }

    #[test]
    fn the_fifth_wrong_attempt_rotates_and_reports_the_cooldown() {
        let mut session = PairingSession::new(creds());
        let now = Instant::now();
        let out = send_wrong(&mut session, 5, now, other_creds());
        assert_eq!(
            out,
            WriteOutcome::LockedOut {
                new_credentials: Some(other_creds()),
                cooldown: Duration::from_secs(30),
            }
        );
    }

    #[test]
    fn during_the_cooldown_even_the_new_correct_secret_is_refused() {
        let mut session = PairingSession::new(creds());
        let t0 = Instant::now();
        send_wrong(&mut session, 5, t0, other_creds());

        let out = session.on_write(
            &frame_with_auth(*other_creds().secret()),
            t0 + Duration::from_secs(1),
            || panic!("no rotation while cooling down"),
        );
        assert_eq!(
            out,
            WriteOutcome::CoolingDown {
                remaining: Duration::from_secs(29)
            }
        );
        // The new fallback code is refused too.
        let pin = auth_from_pin(other_creds().pin()).unwrap();
        assert!(matches!(
            session.on_write(&frame_with_auth(pin), t0 + Duration::from_secs(29), || None),
            WriteOutcome::CoolingDown { .. }
        ));
    }

    #[test]
    fn cooldown_refusals_do_not_count_as_wrong_attempts() {
        let mut session = PairingSession::new(creds());
        let t0 = Instant::now();
        send_wrong(&mut session, 5, t0, other_creds());
        // Far more than 5 garbage frames during the cooldown.
        for i in 0..20u64 {
            let out = session.on_write(
                &frame_with_auth(wrong_auth()),
                t0 + Duration::from_millis(100 * i),
                || panic!("no rotation while cooling down"),
            );
            assert!(matches!(out, WriteOutcome::CoolingDown { .. }));
        }
        let after = t0 + Duration::from_secs(30);
        assert_eq!(
            session.on_write(&frame_with_auth(wrong_auth()), after, || None),
            WriteOutcome::WrongAuth { attempts_left: 4 }
        );
    }

    #[test]
    fn the_new_secret_is_accepted_once_the_cooldown_elapses() {
        let mut session = PairingSession::new(creds());
        let t0 = Instant::now();
        send_wrong(&mut session, 5, t0, other_creds());
        let out = session.on_write(
            &frame_with_auth(*other_creds().secret()),
            t0 + Duration::from_secs(30),
            || None,
        );
        assert_eq!(
            out,
            WriteOutcome::Accepted(request_with_auth(*other_creds().secret()))
        );
    }

    #[test]
    fn the_old_secret_is_dead_after_rotation() {
        let mut session = PairingSession::new(creds());
        let t0 = Instant::now();
        send_wrong(&mut session, 5, t0, other_creds());
        let out = session.on_write(
            &frame_with_auth(*creds().secret()),
            t0 + Duration::from_secs(30),
            || None,
        );
        assert_eq!(out, WriteOutcome::WrongAuth { attempts_left: 4 });
    }

    #[test]
    fn the_second_lockout_uses_a_sixty_second_cooldown() {
        let mut session = PairingSession::new(creds());
        let t0 = Instant::now();
        send_wrong(&mut session, 5, t0, other_creds());
        let t1 = t0 + Duration::from_secs(30);
        let out = send_wrong(&mut session, 5, t1, creds());
        assert_eq!(
            out,
            WriteOutcome::LockedOut {
                new_credentials: Some(creds()),
                cooldown: Duration::from_secs(60),
            }
        );
        assert_eq!(
            session.on_write(
                &frame_with_auth(*creds().secret()),
                t1 + Duration::from_secs(59),
                || None
            ),
            WriteOutcome::CoolingDown {
                remaining: Duration::from_secs(1)
            }
        );
        assert!(matches!(
            session.on_write(
                &frame_with_auth(*creds().secret()),
                t1 + Duration::from_secs(60),
                || None
            ),
            WriteOutcome::Accepted(_)
        ));
    }

    #[test]
    fn a_failed_rotation_leaves_the_gate_locked_and_is_retried_on_the_next_write() {
        let mut session = PairingSession::new(creds());
        let t0 = Instant::now();
        for _ in 0..4 {
            session.on_write(&frame_with_auth(wrong_auth()), t0, || None);
        }
        assert_eq!(
            session.on_write(&frame_with_auth(wrong_auth()), t0, || None),
            WriteOutcome::LockedOut {
                new_credentials: None,
                cooldown: Duration::ZERO,
            }
        );
        // Still locked, no cooldown running: even the correct secret is
        // refused and triggers another rotation attempt.
        let t1 = t0 + Duration::from_secs(5);
        assert_eq!(
            session.on_write(&frame_with_auth(*creds().secret()), t1, || None),
            WriteOutcome::LockedOut {
                new_credentials: None,
                cooldown: Duration::ZERO,
            }
        );
        // The retry succeeds and the cooldown starts at that write.
        assert_eq!(
            session.on_write(&frame_with_auth(wrong_auth()), t1, || Some(other_creds())),
            WriteOutcome::LockedOut {
                new_credentials: Some(other_creds()),
                cooldown: Duration::from_secs(30),
            }
        );
        assert!(matches!(
            session.on_write(&frame_with_auth(*other_creds().secret()), t1, || None),
            WriteOutcome::CoolingDown { .. }
        ));
        assert!(matches!(
            session.on_write(
                &frame_with_auth(*other_creds().secret()),
                t1 + Duration::from_secs(30),
                || None
            ),
            WriteOutcome::Accepted(_)
        ));
    }

    #[test]
    fn there_is_no_permanent_lockout_across_many_rotations() {
        let mut session = PairingSession::new(creds());
        let mut now = Instant::now();
        let mut current = creds();
        for round in 0..12u8 {
            let next = PairingCredentials::from_random_bytes([round + 10; 20]);
            let out = send_wrong(&mut session, 5, now, next.clone());
            assert!(matches!(
                out,
                WriteOutcome::LockedOut {
                    new_credentials: Some(_),
                    ..
                }
            ));
            now += COOLDOWN_MAX;
            current = next;
        }
        assert!(matches!(
            session.on_write(&frame_with_auth(*current.secret()), now, || None),
            WriteOutcome::Accepted(_)
        ));
    }

    // --- redaction, counters --------------------------------------------

    #[test]
    fn credentials_debug_redacts_secret_and_pin() {
        let c = PairingCredentials::from_random_bytes([
            0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5,
            0xA5, 0xA5, 0x39, 0x30, 0x00, 0x00, // 12345 -> pin "012345"
        ]);
        assert_eq!(c.pin(), "012345");
        let out = format!("{c:?}");
        assert!(out.contains("<redacted>"));
        for leak in ["012345", "12345", "a5", "A5", "165", "0xa5", "0xA5", "48"] {
            assert!(!out.contains(leak), "leaked {leak}: {out}");
        }
    }

    #[test]
    fn malformed_and_partial_writes_do_not_consume_attempts() {
        let mut session = PairingSession::new(creds());
        let t0 = Instant::now();
        let frame = frame_with_auth(*creds().secret());
        for i in 0..7u64 {
            // Each round is > STALE_AFTER after the previous one, so the
            // dangling partial frame from the last round is discarded.
            let now = t0 + Duration::from_secs(10 * (i + 1));
            let mut bad = frame.clone();
            bad.push(0);
            assert_eq!(
                session.on_write(&bad, now, || None),
                WriteOutcome::Malformed(FrameError::TooLong)
            );
            assert_eq!(
                session.on_write(&frame[..10], now, || None),
                WriteOutcome::NeedMore
            );
        }
        let now = t0 + Duration::from_secs(1000);
        assert_eq!(
            session.on_write(&frame_with_auth(wrong_auth()), now, || None),
            WriteOutcome::WrongAuth { attempts_left: 4 }
        );
        assert_eq!(
            session.on_write(&frame, now, || None),
            WriteOutcome::Accepted(request_with_auth(*creds().secret()))
        );
    }

    #[test]
    fn malformed_and_partial_writes_are_unaffected_by_the_cooldown() {
        let mut session = PairingSession::new(creds());
        let t0 = Instant::now();
        send_wrong(&mut session, 5, t0, other_creds());
        let now = t0 + Duration::from_secs(1);
        let frame = frame_with_auth(*other_creds().secret());
        let mut bad = frame.clone();
        bad.push(0);
        assert_eq!(
            session.on_write(&bad, now, || None),
            WriteOutcome::Malformed(FrameError::TooLong)
        );
        assert_eq!(
            session.on_write(&frame[..10], now, || None),
            WriteOutcome::NeedMore
        );
    }
}
