//! Wire format for first-run family-member pairing over BLE, shared by the
//! kiosk (BLE peripheral) and family-app (BLE central) so both sides use
//! the same encoder/decoder. See the family-app design spec, "Identity &
//! Bootstrap".
//!
//! One writable characteristic carries a single frame:
//!
//! ```text
//! [ 16 bytes: auth field ]
//! [ 32 bytes: the family member's pubkey ]
//! [ 1 byte: N, the name length in bytes, 1..=32 ]
//! [ N bytes: UTF-8 name ]
//! ```
//!
//! The auth field is either the 16-byte secret carried by the QR code the
//! kiosk shows ([`encode_qr_payload`]) or the kiosk's 6-digit fallback code
//! zero-padded to 16 bytes ([`auth_from_pin`]); the kiosk accepts either.
//!
//! A central may deliver the frame in one write or several (an ATT payload
//! can be as small as 20 bytes), so the receiving side reassembles with
//! [`FrameAssembler`].

use std::time::{Duration, Instant};

use solana_pubkey::Pubkey;

/// BLE GATT identifiers as raw `u128`, so this crate depends on no BLE
/// stack: the kiosk converts with `bluer::Uuid::from_u128`, family-app with
/// `uuid::Uuid::from_u128`.
pub const SERVICE_UUID_U128: u128 = 0x8f1a0000_5b1e_4c6a_9a7e_0d2f3a1b4c5d;
pub const PAIRING_CHARACTERISTIC_UUID_U128: u128 = 0x8f1a0001_5b1e_4c6a_9a7e_0d2f3a1b4c5d;

pub const AUTH_LEN: usize = 16;
pub const PIN_LEN: usize = 6;
pub const PUBKEY_LEN: usize = 32;
/// Mirrors `family_vault::state::BENEFICIARY_NAME_LEN` (a test pins them
/// together) — the program rejects longer names.
pub const MAX_NAME_LEN: usize = 32;
const HEADER_LEN: usize = AUTH_LEN + PUBKEY_LEN + 1;

/// Prefix (and version) of the text encoded in the kiosk's pairing QR.
pub const QR_PREFIX: &str = "fvpair1:";

/// A partially-received frame older than this is dropped before the next
/// write is appended, so a central that gave up halfway cannot corrupt the
/// next attempt.
pub const STALE_AFTER: Duration = Duration::from_secs(3);

pub type AuthField = [u8; AUTH_LEN];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingRequest {
    pub auth: AuthField,
    pub pubkey: Pubkey,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The fallback code is not exactly six ASCII digits.
    BadPin,
    NameEmpty,
    NameTooLong,
    NameNotUtf8,
    /// More bytes arrived than the declared name length allows.
    TooLong,
}

/// The fallback code as an auth field: the six ASCII digits, zero-padded.
pub fn auth_from_pin(pin: &str) -> Result<AuthField, FrameError> {
    if pin.len() != PIN_LEN || !pin.bytes().all(|b| b.is_ascii_digit()) {
        return Err(FrameError::BadPin);
    }
    let mut field = [0u8; AUTH_LEN];
    field[..PIN_LEN].copy_from_slice(pin.as_bytes());
    Ok(field)
}

/// The text the kiosk encodes in its pairing QR: [`QR_PREFIX`] followed by
/// the secret as 32 lowercase hex characters.
pub fn encode_qr_payload(secret: &AuthField) -> String {
    let hex: String = secret.iter().map(|b| format!("{b:02x}")).collect();
    format!("{QR_PREFIX}{hex}")
}

/// Parses text scanned from the kiosk's QR. Strict: exact prefix, exactly
/// 32 hex digits (either case).
pub fn decode_qr_payload(text: &str) -> Option<AuthField> {
    let hex = text.strip_prefix(QR_PREFIX)?;
    if hex.len() != AUTH_LEN * 2 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut secret = [0u8; AUTH_LEN];
    for (i, byte) in secret.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(secret)
}

pub fn encode_frame(request: &PairingRequest) -> Result<Vec<u8>, FrameError> {
    let name = request.name.as_bytes();
    if name.is_empty() {
        return Err(FrameError::NameEmpty);
    }
    if name.len() > MAX_NAME_LEN {
        return Err(FrameError::NameTooLong);
    }
    let mut frame = Vec::with_capacity(HEADER_LEN + name.len());
    frame.extend_from_slice(&request.auth);
    frame.extend_from_slice(&request.pubkey.to_bytes());
    frame.push(name.len() as u8);
    frame.extend_from_slice(name);
    Ok(frame)
}

/// Reassembles a frame from however many writes the central used.
#[derive(Debug, Default)]
pub struct FrameAssembler {
    buf: Vec<u8>,
    last_write: Option<Instant>,
}

impl FrameAssembler {
    /// Feeds one write. `Ok(Some(_))` means a complete, valid frame was
    /// assembled; `Ok(None)` means more bytes are needed; `Err(_)` means
    /// the frame is malformed. After `Ok(Some(_))` or `Err(_)` the buffer
    /// is cleared, so the next write starts a fresh frame. The auth field
    /// is opaque here — checking it is the receiver's job.
    pub fn push(
        &mut self,
        chunk: &[u8],
        now: Instant,
    ) -> Result<Option<PairingRequest>, FrameError> {
        if let Some(last) = self.last_write {
            if now.saturating_duration_since(last) > STALE_AFTER {
                self.buf.clear();
            }
        }
        self.last_write = Some(now);
        self.buf.extend_from_slice(chunk);

        let parsed = self.try_parse();
        if !matches!(parsed, Ok(None)) {
            self.buf.clear();
        }
        parsed
    }

    fn try_parse(&self) -> Result<Option<PairingRequest>, FrameError> {
        if self.buf.len() < HEADER_LEN {
            return Ok(None);
        }
        let name_len = self.buf[HEADER_LEN - 1] as usize;
        if name_len == 0 {
            return Err(FrameError::NameEmpty);
        }
        if name_len > MAX_NAME_LEN {
            return Err(FrameError::NameTooLong);
        }
        let total = HEADER_LEN + name_len;
        if self.buf.len() < total {
            return Ok(None);
        }
        if self.buf.len() > total {
            return Err(FrameError::TooLong);
        }

        let mut auth = [0u8; AUTH_LEN];
        auth.copy_from_slice(&self.buf[..AUTH_LEN]);

        let mut pubkey_bytes = [0u8; PUBKEY_LEN];
        pubkey_bytes.copy_from_slice(&self.buf[AUTH_LEN..AUTH_LEN + PUBKEY_LEN]);

        let name = std::str::from_utf8(&self.buf[HEADER_LEN..total])
            .map_err(|_| FrameError::NameNotUtf8)?
            .to_string();

        Ok(Some(PairingRequest {
            auth,
            pubkey: Pubkey::new_from_array(pubkey_bytes),
            name,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn sample() -> PairingRequest {
        PairingRequest {
            auth: [0xAB; AUTH_LEN],
            pubkey: Pubkey::new_from_array([7u8; 32]),
            name: "Внук Саша".to_string(),
        }
    }

    fn valid_frame() -> Vec<u8> {
        encode_frame(&sample()).unwrap()
    }

    // --- frame assembly -------------------------------------------------

    #[test]
    fn a_frame_delivered_in_one_write_round_trips() {
        let mut assembler = FrameAssembler::default();
        let out = assembler.push(&valid_frame(), Instant::now()).unwrap();
        assert_eq!(out, Some(sample()));
    }

    #[test]
    fn a_frame_split_into_20_byte_writes_round_trips() {
        let mut assembler = FrameAssembler::default();
        let now = Instant::now();
        let frame = valid_frame();
        let mut chunks = frame.chunks(20).peekable();
        while let Some(chunk) = chunks.next() {
            let out = assembler.push(chunk, now).unwrap();
            if chunks.peek().is_some() {
                assert_eq!(out, None, "must ask for more until the last chunk");
            } else {
                assert_eq!(out, Some(sample()));
            }
        }
    }

    #[test]
    fn a_frame_delivered_one_byte_at_a_time_round_trips() {
        let mut assembler = FrameAssembler::default();
        let now = Instant::now();
        let frame = valid_frame();
        let last = frame.len() - 1;
        for (i, byte) in frame.iter().enumerate() {
            let out = assembler.push(&[*byte], now).unwrap();
            if i == last {
                assert_eq!(out, Some(sample()));
            } else {
                assert_eq!(out, None);
            }
        }
    }

    #[test]
    fn a_stale_partial_frame_is_discarded_before_the_next_write() {
        let mut assembler = FrameAssembler::default();
        let t0 = Instant::now();
        let frame = valid_frame();
        assert_eq!(assembler.push(&frame[..10], t0).unwrap(), None);
        // Without stale-discarding, these 10 leftover bytes plus a full
        // frame would overshoot and error out as TooLong.
        let later = t0 + STALE_AFTER + Duration::from_secs(1);
        assert_eq!(assembler.push(&frame, later).unwrap(), Some(sample()));
    }

    #[test]
    fn a_fresh_partial_frame_is_kept() {
        let mut assembler = FrameAssembler::default();
        let t0 = Instant::now();
        let frame = valid_frame();
        assert_eq!(assembler.push(&frame[..10], t0).unwrap(), None);
        let soon = t0 + Duration::from_secs(1);
        assert_eq!(assembler.push(&frame[10..], soon).unwrap(), Some(sample()));
    }

    fn header_with_name_len(n: u8) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.extend_from_slice(&[0xAB; AUTH_LEN]);
        frame.extend_from_slice(&[7u8; 32]);
        frame.push(n);
        frame
    }

    #[test]
    fn a_zero_length_name_is_rejected() {
        let mut assembler = FrameAssembler::default();
        assert_eq!(
            assembler.push(&header_with_name_len(0), Instant::now()),
            Err(FrameError::NameEmpty)
        );
    }

    #[test]
    fn a_name_longer_than_32_bytes_is_rejected_as_soon_as_the_header_is_complete() {
        let mut assembler = FrameAssembler::default();
        assert_eq!(
            assembler.push(&header_with_name_len(33), Instant::now()),
            Err(FrameError::NameTooLong)
        );
    }

    #[test]
    fn trailing_bytes_after_a_complete_frame_are_rejected() {
        let mut frame = valid_frame();
        frame.push(0);
        let mut assembler = FrameAssembler::default();
        assert_eq!(
            assembler.push(&frame, Instant::now()),
            Err(FrameError::TooLong)
        );
    }

    #[test]
    fn a_name_that_is_not_utf8_is_rejected() {
        let mut frame = header_with_name_len(2);
        frame.extend_from_slice(&[0xff, 0xfe]);
        let mut assembler = FrameAssembler::default();
        assert_eq!(
            assembler.push(&frame, Instant::now()),
            Err(FrameError::NameNotUtf8)
        );
    }

    #[test]
    fn a_bad_frame_does_not_poison_the_next_one() {
        let mut assembler = FrameAssembler::default();
        let now = Instant::now();
        let mut bad = valid_frame();
        bad.push(0);
        assert_eq!(assembler.push(&bad, now), Err(FrameError::TooLong));
        assert_eq!(assembler.push(&valid_frame(), now).unwrap(), Some(sample()));
    }

    #[test]
    fn encode_rejects_bad_names() {
        let mut req = sample();
        req.name = String::new();
        assert_eq!(encode_frame(&req), Err(FrameError::NameEmpty));

        let mut req = sample();
        req.name = "a".repeat(33);
        assert_eq!(encode_frame(&req), Err(FrameError::NameTooLong));
    }

    // --- fallback code as an auth field ---------------------------------

    #[test]
    fn auth_from_pin_is_the_six_digits_zero_padded() {
        let field = auth_from_pin("482913").unwrap();
        assert_eq!(&field[..6], b"482913");
        assert!(field[6..].iter().all(|b| *b == 0));
    }

    #[test]
    fn auth_from_pin_rejects_anything_but_six_ascii_digits() {
        assert_eq!(auth_from_pin("12345"), Err(FrameError::BadPin));
        assert_eq!(auth_from_pin("1234567"), Err(FrameError::BadPin));
        assert_eq!(auth_from_pin("12345x"), Err(FrameError::BadPin));
        assert_eq!(auth_from_pin("١٢٣٤٥٦"), Err(FrameError::BadPin));
        assert_eq!(auth_from_pin(""), Err(FrameError::BadPin));
    }

    // --- QR payload -----------------------------------------------------

    #[test]
    fn a_qr_payload_round_trips() {
        let secret: AuthField = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd,
            0xfe, 0xff,
        ];
        let text = encode_qr_payload(&secret);
        assert_eq!(text, "fvpair1:0001020304050607f8f9fafbfcfdfeff");
        assert_eq!(decode_qr_payload(&text), Some(secret));
    }

    #[test]
    fn decode_accepts_uppercase_hex() {
        let secret: AuthField = [0xAB; AUTH_LEN];
        let upper = format!("fvpair1:{}", "AB".repeat(16));
        assert_eq!(decode_qr_payload(&upper), Some(secret));
    }

    #[test]
    fn decode_rejects_malformed_payloads() {
        let good = format!("fvpair1:{}", "ab".repeat(16));
        assert!(decode_qr_payload(&good).is_some());

        assert_eq!(decode_qr_payload(&"ab".repeat(16)), None, "missing prefix");
        assert_eq!(
            decode_qr_payload(&format!("fvpair2:{}", "ab".repeat(16))),
            None,
            "unknown version"
        );
        assert_eq!(decode_qr_payload("fvpair1:abcd"), None, "too short");
        assert_eq!(
            decode_qr_payload(&format!("fvpair1:{}", "ab".repeat(17))),
            None,
            "too long"
        );
        assert_eq!(
            decode_qr_payload(&format!("fvpair1:{}", "g".repeat(32))),
            None,
            "not hex"
        );
        // `u8::from_str_radix` alone would accept a leading '+'.
        assert_eq!(
            decode_qr_payload(&format!("fvpair1:{}", "+f".repeat(16))),
            None,
            "sign characters are not hex digits"
        );
    }

    // --- misc -----------------------------------------------------------

    #[test]
    fn max_name_len_matches_the_on_chain_limit() {
        assert_eq!(MAX_NAME_LEN, family_vault::state::BENEFICIARY_NAME_LEN);
    }

    // Manual helper for testing the kiosk with a phone BLE tool (e.g. nRF
    // Connect): prints the hex of a frame to paste into a write. Give it
    // EITHER the text scanned from the kiosk's QR OR the fallback code:
    //   PAIR_QR='fvpair1:…' PAIR_PUBKEY=<base58> PAIR_NAME="Внук Саша" \
    //   cargo test -p family-vault-client print_frame_hex -- --ignored --nocapture
    //   PAIR_PIN=123456 PAIR_PUBKEY=<base58> PAIR_NAME="Внук Саша" \
    //   cargo test -p family-vault-client print_frame_hex -- --ignored --nocapture
    #[test]
    #[ignore = "manual helper: reads PAIR_QR or PAIR_PIN, plus PAIR_PUBKEY and PAIR_NAME"]
    fn print_frame_hex() {
        let auth = match std::env::var("PAIR_QR") {
            Ok(text) => decode_qr_payload(&text).expect("PAIR_QR is not a valid pairing QR text"),
            Err(_) => auth_from_pin(&std::env::var("PAIR_PIN").expect("set PAIR_QR or PAIR_PIN"))
                .expect("PAIR_PIN must be six digits"),
        };
        let request = PairingRequest {
            auth,
            pubkey: std::env::var("PAIR_PUBKEY")
                .expect("set PAIR_PUBKEY")
                .parse()
                .expect("PAIR_PUBKEY must be a valid base58 pubkey"),
            name: std::env::var("PAIR_NAME").expect("set PAIR_NAME"),
        };
        let frame = encode_frame(&request).expect("valid input");
        let hex: String = frame.iter().map(|b| format!("{b:02X}")).collect();
        println!("{hex}");
    }
}
