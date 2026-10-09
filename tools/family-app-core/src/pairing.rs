//! BLE pairing from the family member's side (the GATT central). The kiosk,
//! in first-run setup, advertises one pairing service with one writable
//! characteristic (see `kiosk-gui/src/ble.rs`); this module scans for it,
//! connects, and writes the shared frame (`family_vault_client::pairing`)
//! WITH response, so a wrong or expired code comes back as an error rather
//! than silently vanishing.

use std::fmt;
use std::future::Future;
use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Manager, Peripheral};
use family_vault_client::pairing::{
    decode_qr_payload, encode_frame, PairingRequest, PAIRING_CHARACTERISTIC_UUID_U128,
    SERVICE_UUID_U128,
};
use solana_pubkey::Pubkey;
use uuid::Uuid;

const SERVICE_UUID: Uuid = Uuid::from_u128(SERVICE_UUID_U128);
const PAIRING_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(PAIRING_CHARACTERISTIC_UUID_U128);

/// How long to look for the kiosk before giving up.
const SCAN_TIMEOUT: Duration = Duration::from_secs(15);
const SCAN_POLL_INTERVAL: Duration = Duration::from_millis(250);
/// Bound for every other single BLE step (stack start-up, connect, service
/// discovery, the write itself) so nothing can hang forever.
const STEP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingError {
    /// The scanned QR text is not a kiosk pairing code. No Bluetooth
    /// activity happened.
    BadQrText,
    /// The name cannot be encoded (empty, too long, control characters).
    BadName(String),
    /// No Bluetooth adapter on this machine.
    NoAdapter,
    /// No peripheral advertising the pairing service appeared within the
    /// scan timeout.
    KioskNotFound,
    /// Connecting to the kiosk, or discovering its services, failed.
    ConnectFailed(String),
    /// The kiosk refused the write (wrong or expired code, malformed
    /// frame) or the write did not complete.
    WriteRejected(String),
    /// Anything else from the BLE stack (Bluetooth off, permission
    /// denied, timeouts, a kiosk without the pairing characteristic, ...).
    Ble(String),
}

impl fmt::Display for PairingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadQrText => write!(f, "not a valid kiosk pairing code"),
            Self::BadName(e) => write!(f, "invalid name: {e}"),
            Self::NoAdapter => write!(f, "no Bluetooth adapter available"),
            Self::KioskNotFound => write!(f, "no kiosk in pairing mode found nearby"),
            Self::ConnectFailed(e) => write!(f, "could not connect to the kiosk: {e}"),
            Self::WriteRejected(e) => write!(f, "the kiosk rejected the pairing request: {e}"),
            Self::Ble(e) => write!(f, "bluetooth error: {e}"),
        }
    }
}

impl std::error::Error for PairingError {}

fn ble(e: impl fmt::Display) -> PairingError {
    PairingError::Ble(e.to_string())
}

/// Runs `fut` for at most `STEP_TIMEOUT`; a timeout becomes `on_timeout`.
async fn step<T, E: fmt::Display>(
    fut: impl Future<Output = Result<T, E>>,
    on_error: impl FnOnce(String) -> PairingError,
) -> Result<T, PairingError> {
    match tokio::time::timeout(STEP_TIMEOUT, fut).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(on_error(e.to_string())),
        Err(_) => Err(on_error("timed out".to_string())),
    }
}

/// Pairs this app's identity with a kiosk in first-run setup. Blocks the
/// calling thread for up to roughly a minute; safe to call from any thread,
/// including one that is inside an async runtime (the work runs on its own
/// short-lived thread and runtime).
///
/// `qr_text` is the text scanned from the kiosk's QR code. A bad code is
/// reported before any Bluetooth activity.
pub fn pair_with_kiosk(qr_text: &str, name: &str, my_pubkey: Pubkey) -> Result<(), PairingError> {
    let auth = decode_qr_payload(qr_text).ok_or(PairingError::BadQrText)?;
    let frame = encode_frame(&PairingRequest {
        auth,
        pubkey: my_pubkey,
        name: name.to_string(),
    })
    .map_err(|e| PairingError::BadName(format!("{e:?}")))?;

    // A dedicated thread has no ambient tokio context, so `block_on` cannot
    // panic with "cannot start a runtime from within a runtime" even if the
    // caller is itself async.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("family-app-pairing".to_string())
            .spawn_scoped(scope, || build_runtime()?.block_on(run(&frame)))
            .map_err(ble)?
            .join()
            .unwrap_or_else(|_| Err(PairingError::Ble("pairing thread panicked".to_string())))
    })
}

/// The BlueZ backend talks to D-Bus through `dbus-tokio`, which registers
/// file descriptors with the runtime's IO driver. A runtime built with only
/// `enable_time()` panics there ("IO is disabled"), so every driver is on.
fn build_runtime() -> Result<tokio::runtime::Runtime, PairingError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(ble)
}

async fn run(frame: &[u8]) -> Result<(), PairingError> {
    let manager = step(Manager::new(), PairingError::Ble).await?;
    let adapter = step(manager.adapters(), PairingError::Ble)
        .await?
        .into_iter()
        .next()
        .ok_or(PairingError::NoAdapter)?;

    step(
        adapter.start_scan(ScanFilter {
            services: vec![SERVICE_UUID],
        }),
        PairingError::Ble,
    )
    .await?;
    let found = find_kiosks(&adapter).await;
    // Best effort: a failure to stop scanning must not mask the outcome.
    let _ = tokio::time::timeout(STEP_TIMEOUT, adapter.stop_scan()).await;

    // BlueZ also reports devices it has cached, which may be out of range, so
    // a failed connection moves on to the next candidate. Any other outcome
    // (success, or the kiosk rejecting the code) is final: a kiosk answered.
    let mut last_error = PairingError::KioskNotFound;
    for kiosk in found? {
        let result = talk_to_kiosk(&kiosk, frame).await;
        let _ = tokio::time::timeout(STEP_TIMEOUT, kiosk.disconnect()).await;
        match result {
            Err(PairingError::ConnectFailed(e)) => last_error = PairingError::ConnectFailed(e),
            other => return other,
        }
    }
    Err(last_error)
}

/// Polls the adapter's discovered devices until at least one advertises the
/// pairing service or `SCAN_TIMEOUT` passes, and returns every such device.
/// The service filter is applied by BlueZ when scanning, and re-checked here
/// because other platforms (and cached devices) may report peripherals the
/// filter would have excluded.
async fn find_kiosks(
    adapter: &impl Central<Peripheral = Peripheral>,
) -> Result<Vec<Peripheral>, PairingError> {
    let deadline = tokio::time::Instant::now() + SCAN_TIMEOUT;
    loop {
        let mut found = Vec::new();
        for peripheral in step(adapter.peripherals(), PairingError::Ble).await? {
            // Never let the per-device lookups run past the scan deadline.
            let budget = deadline
                .saturating_duration_since(tokio::time::Instant::now())
                .min(STEP_TIMEOUT);
            let advertised = matches!(
                tokio::time::timeout(budget, peripheral.properties()).await,
                Ok(Ok(Some(props))) if props.services.contains(&SERVICE_UUID)
            );
            if advertised {
                found.push(peripheral);
            }
        }
        if !found.is_empty() {
            return Ok(found);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(PairingError::KioskNotFound);
        }
        tokio::time::sleep(SCAN_POLL_INTERVAL).await;
    }
}

async fn talk_to_kiosk(kiosk: &Peripheral, frame: &[u8]) -> Result<(), PairingError> {
    step(kiosk.connect(), PairingError::ConnectFailed).await?;
    step(kiosk.discover_services(), PairingError::ConnectFailed).await?;
    let characteristic = kiosk
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == PAIRING_CHARACTERISTIC_UUID)
        .ok_or_else(|| ble("the device has no pairing characteristic"))?;
    // WithResponse: the kiosk answers a wrong/expired code with an ATT
    // error, which btleplug returns as an `Err` here.
    step(
        kiosk.write(&characteristic, frame, WriteType::WithResponse),
        PairingError::WriteRejected,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use family_vault_client::pairing::decode_qr_payload;
    use solana_pubkey::Pubkey;

    #[test]
    fn rejects_qr_text_that_does_not_decode() {
        let err = pair_with_kiosk(
            "not a real pairing code",
            "Внук Саша",
            Pubkey::new_from_array([1u8; 32]),
        );
        assert!(matches!(err, Err(PairingError::BadQrText)));
    }

    // The BlueZ backend registers file descriptors with the runtime. This
    // fails with "IO is disabled" if the runtime is built without the IO
    // driver, which no other offline test would notice.
    #[cfg(unix)]
    #[test]
    fn the_pairing_runtime_can_register_file_descriptors() {
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        a.set_nonblocking(true).unwrap();
        let runtime = build_runtime().unwrap();
        runtime.block_on(async {
            tokio::io::unix::AsyncFd::new(a).expect("IO driver must be enabled");
        });
    }

    #[test]
    fn a_valid_qr_payload_decodes_before_any_bluetooth_activity() {
        let secret = [7u8; family_vault_client::pairing::AUTH_LEN];
        let text = family_vault_client::pairing::encode_qr_payload(&secret);
        assert_eq!(decode_qr_payload(&text), Some(secret));
    }

    // Requires a real kiosk in first-run pairing mode, in Bluetooth range,
    // showing a QR — scan it with a phone or `zbarimg`/similar to get the
    // text. Run manually with
    // `PAIR_QR='fvpair1:...' cargo test -p family-app-core pairing::tests::pairs_with_a_real_kiosk -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a real kiosk in range, in first-run pairing mode"]
    fn pairs_with_a_real_kiosk() {
        let qr_text = std::env::var("PAIR_QR").expect("set PAIR_QR to the kiosk's scanned QR text");
        let my_pubkey = Pubkey::new_unique();
        pair_with_kiosk(&qr_text, "Тестовый Аккаунт", my_pubkey).unwrap();
    }
}
