//! BLE peripheral side of first-run family-member pairing (Linux/BlueZ only,
//! via `bluer`). While `wait_for_pairing` runs, the kiosk advertises one
//! GATT service with one writable characteristic; a family member's app
//! writes a frame (see `family_vault_client::pairing`) and the kiosk accepts
//! it only if it carries the QR secret or fallback code currently shown on
//! screen. All the decision-making lives in
//! `crate::pairing::PairingSession`; this module only adapts it to BlueZ and
//! maps outcomes to ATT errors.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use bluer::adv::Advertisement;
use bluer::gatt::local::{
    Application, Characteristic, CharacteristicWrite, CharacteristicWriteMethod, ReqError, Service,
};
use bluer::Uuid;
use family_vault_client::pairing::{
    PairingRequest, PAIRING_CHARACTERISTIC_UUID_U128, SERVICE_UUID_U128,
};
use futures::FutureExt;

use crate::pairing::{generate_credentials, PairingCredentials, PairingSession, WriteOutcome};

const SERVICE_UUID: Uuid = Uuid::from_u128(SERVICE_UUID_U128);
const PAIRING_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(PAIRING_CHARACTERISTIC_UUID_U128);

/// Called (from the BLE runtime thread) with the new credentials whenever
/// the kiosk rotates them after too many wrong attempts, so the UI can
/// redraw the QR and code.
pub type CredentialsRotated = Arc<dyn Fn(&PairingCredentials) + Send + Sync>;

/// Bluetooth is not usable (no adapter, `bluetoothd` not running, ...).
/// The setup flow treats this as "skip the pairing step", never as fatal.
#[derive(Debug)]
pub struct BleError(pub String);

impl fmt::Display for BleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bluetooth unavailable: {}", self.0)
    }
}

impl std::error::Error for BleError {}

fn unavailable(e: bluer::Error) -> BleError {
    BleError(e.to_string())
}

/// Blocks the calling thread while advertising, until either a request
/// carrying valid credentials arrives (`Ok(Some(request))`) or `cancel` is
/// set (`Ok(None)`). The advertisement and GATT application are removed
/// before this returns.
pub fn wait_for_pairing(
    credentials: PairingCredentials,
    on_rotated: CredentialsRotated,
    cancel: &AtomicBool,
) -> Result<Option<PairingRequest>, BleError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| BleError(e.to_string()))?;
    runtime.block_on(run(credentials, on_rotated, cancel))
}

async fn run(
    credentials: PairingCredentials,
    on_rotated: CredentialsRotated,
    cancel: &AtomicBool,
) -> Result<Option<PairingRequest>, BleError> {
    let session = bluer::Session::new().await.map_err(unavailable)?;
    let adapter = session.default_adapter().await.map_err(unavailable)?;
    adapter.set_powered(true).await.map_err(unavailable)?;

    let advertisement = Advertisement {
        service_uuids: vec![SERVICE_UUID].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some("FamilyVault".to_string()),
        ..Default::default()
    };
    let advertisement_handle = adapter.advertise(advertisement).await.map_err(unavailable)?;

    let state = Arc::new(Mutex::new(PairingSession::new(credentials)));
    let (accepted_tx, accepted_rx) = mpsc::channel::<PairingRequest>();

    let write_state = state.clone();
    let write_tx = accepted_tx.clone();
    let write_rotated = on_rotated.clone();
    let application = Application {
        services: vec![Service {
            uuid: SERVICE_UUID,
            primary: true,
            characteristics: vec![Characteristic {
                uuid: PAIRING_CHARACTERISTIC_UUID,
                write: Some(CharacteristicWrite {
                    write: true,
                    write_without_response: true,
                    method: CharacteristicWriteMethod::Fun(Box::new(move |value, _req| {
                        let state = write_state.clone();
                        let tx = write_tx.clone();
                        let rotated = write_rotated.clone();
                        async move { handle_write(&state, &tx, &rotated, &value) }.boxed()
                    })),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let application_handle = adapter
        .serve_gatt_application(application)
        .await
        .map_err(unavailable)?;

    let outcome = loop {
        if cancel.load(Ordering::Relaxed) {
            break None;
        }
        match accepted_rx.try_recv() {
            Ok(request) => break Some(request),
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => break None,
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };

    // Tear down explicitly and give BlueZ a moment to process the removal
    // before the runtime (and with it the D-Bus connection) goes away.
    drop(application_handle);
    drop(advertisement_handle);
    tokio::time::sleep(Duration::from_millis(300)).await;

    Ok(outcome)
}

/// Maps one BLE write to an ATT response. Wrong credentials and malformed
/// frames get a real ATT error so the central can tell the user what went
/// wrong. Never logs the secret or the code.
fn handle_write(
    session: &Mutex<PairingSession>,
    accepted: &mpsc::Sender<PairingRequest>,
    on_rotated: &CredentialsRotated,
    value: &[u8],
) -> Result<(), ReqError> {
    let Ok(mut session) = session.lock() else {
        return Err(ReqError::Failed);
    };
    let outcome = session.on_write(value, Instant::now(), || generate_credentials().ok());
    match outcome {
        WriteOutcome::NeedMore => Ok(()),
        WriteOutcome::Accepted(request) => {
            let _ = accepted.send(request);
            Ok(())
        }
        WriteOutcome::WrongAuth { attempts_left } => {
            eprintln!("[kiosk-gui] pairing: wrong credentials ({attempts_left} attempts left)");
            Err(ReqError::NotAuthorized)
        }
        WriteOutcome::LockedOut { new_credentials } => {
            eprintln!("[kiosk-gui] pairing: too many wrong attempts, rotating credentials");
            if let Some(credentials) = new_credentials {
                on_rotated(&credentials);
            }
            Err(ReqError::NotAuthorized)
        }
        WriteOutcome::Malformed(e) => {
            eprintln!("[kiosk-gui] pairing: malformed frame: {e:?}");
            Err(ReqError::InvalidValueLength)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use family_vault_client::pairing::{auth_from_pin, encode_frame, AuthField};
    use solana_pubkey::Pubkey;
    use std::sync::atomic::AtomicBool;

    fn creds() -> PairingCredentials {
        PairingCredentials::from_random_bytes([1u8; 20])
    }

    fn frame(auth: AuthField) -> Vec<u8> {
        encode_frame(&PairingRequest {
            auth,
            pubkey: Pubkey::new_from_array([7u8; 32]),
            name: "Внук Саша".to_string(),
        })
        .unwrap()
    }

    fn wrong() -> AuthField {
        [0xEE; 16]
    }

    struct Fixture {
        session: Mutex<PairingSession>,
        tx: mpsc::Sender<PairingRequest>,
        rx: mpsc::Receiver<PairingRequest>,
        callback: CredentialsRotated,
        rotated: Arc<Mutex<Vec<PairingCredentials>>>,
    }

    fn fixture() -> Fixture {
        let rotated = Arc::new(Mutex::new(Vec::new()));
        let sink = rotated.clone();
        let callback: CredentialsRotated =
            Arc::new(move |c: &PairingCredentials| sink.lock().unwrap().push(c.clone()));
        let (tx, rx) = mpsc::channel();
        Fixture {
            session: Mutex::new(PairingSession::new(creds())),
            tx,
            rx,
            callback,
            rotated,
        }
    }

    fn write(f: &Fixture, value: &[u8]) -> Result<(), ReqError> {
        handle_write(&f.session, &f.tx, &f.callback, value)
    }

    #[test]
    fn a_correct_qr_secret_is_acknowledged_and_forwarded() {
        let f = fixture();
        assert!(write(&f, &frame(*creds().secret())).is_ok());
        let request = f.rx.try_recv().expect("accepted request must be forwarded");
        assert_eq!(request.name, "Внук Саша");
    }

    #[test]
    fn a_correct_fallback_code_is_acknowledged_and_forwarded() {
        let f = fixture();
        let field = auth_from_pin(creds().pin()).unwrap();
        assert!(write(&f, &frame(field)).is_ok());
        assert!(f.rx.try_recv().is_ok());
    }

    #[test]
    fn a_partial_write_is_acknowledged_but_not_forwarded() {
        let f = fixture();
        let full = frame(*creds().secret());
        assert!(write(&f, &full[..20]).is_ok());
        assert!(f.rx.try_recv().is_err());
    }

    #[test]
    fn wrong_credentials_are_rejected_with_not_authorized_and_not_forwarded() {
        let f = fixture();
        let res = write(&f, &frame(wrong()));
        assert!(matches!(res, Err(ReqError::NotAuthorized)));
        assert!(f.rx.try_recv().is_err());
    }

    #[test]
    fn a_malformed_frame_is_rejected_with_invalid_value_length() {
        let f = fixture();
        let mut bad = frame(*creds().secret());
        bad.push(0); // trailing byte
        let res = write(&f, &bad);
        assert!(matches!(res, Err(ReqError::InvalidValueLength)));
    }

    #[test]
    fn lockout_rotates_the_credentials_and_tells_the_ui() {
        let f = fixture();
        for _ in 0..5 {
            let res = write(&f, &frame(wrong()));
            assert!(matches!(res, Err(ReqError::NotAuthorized)));
        }
        let new_creds = {
            let seen = f.rotated.lock().unwrap();
            assert_eq!(seen.len(), 1, "UI must be told about exactly one rotation");
            seen[0].clone()
        };
        assert_ne!(new_creds.secret(), creds().secret());

        // The old QR secret is dead; the freshly shown one works.
        assert!(matches!(
            write(&f, &frame(*creds().secret())),
            Err(ReqError::NotAuthorized)
        ));
        assert!(write(&f, &frame(*new_creds.secret())).is_ok());
        assert!(f.rx.try_recv().is_ok());
    }

    // The two tests below register a real advertisement and GATT
    // application with BlueZ, so they need a Bluetooth adapter and a running
    // bluetoothd. Run manually:
    //   cargo test -p kiosk-gui ble::tests -- --ignored --nocapture
    #[test]
    #[ignore = "needs a Bluetooth adapter and a running bluetoothd"]
    fn returns_none_immediately_when_cancel_is_already_set() {
        let cancel = AtomicBool::new(true);
        let out = wait_for_pairing(creds(), Arc::new(|_| {}), &cancel).unwrap();
        assert_eq!(out, None);
    }

    #[test]
    #[ignore = "needs a Bluetooth adapter and a running bluetoothd"]
    fn stops_when_cancel_is_set_while_advertising() {
        let cancel = AtomicBool::new(false);
        let started = std::time::Instant::now();
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(1500));
                cancel.store(true, Ordering::Relaxed);
            });
            let out = wait_for_pairing(creds(), Arc::new(|_| {}), &cancel).unwrap();
            assert_eq!(out, None);
        });
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
