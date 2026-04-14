use crate::efi_vars;
use pgp::{Deserializable, SignedPublicKey, StandaloneSignature};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Cursor, ErrorKind, Read, Write};
use std::path::Path;

const SLOT_A: u8 = 0;
const SLOT_B: u8 = 1;
const SLOT_C: u8 = 2;

const STATUS_STAGING: u8 = 0;
const NOTIFY_PATH: &str = "/run/stratsup-notify";

struct SlotTarget {
    slot_id: u8,
    status_var: &'static str,
    block_device: &'static str,
}

struct UpdateConfig {
    image_url: String,
    manifest_url: String,
    signature_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorState {
    Dormant,
    Downloading,
    ReadyToSwitch,
    Pivoting,
    PivotFailed,
}

pub struct Supervisor {
    state: SupervisorState,
    shutdown_requested: bool,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            state: SupervisorState::Dormant,
            shutdown_requested: false,
        }
    }

    pub fn run_once(&mut self) -> Result<(), String> {
        match self.state {
            SupervisorState::Dormant => self.handle_dormant(),
            SupervisorState::Downloading => self.handle_downloading(),
            SupervisorState::ReadyToSwitch => self.handle_ready_to_switch(),
            SupervisorState::Pivoting => self.handle_pivoting(),
            SupervisorState::PivotFailed => self.handle_pivot_failed(),
        }
    }

    fn handle_dormant(&mut self) -> Result<(), String> {
        use std::io::Read;
        use std::os::unix::net::UnixListener;

        const SOCK_PATH: &str = "/run/stratsup.sock";

        if let Err(err) = fs::create_dir_all("/run") {
            return Err(format!(
                "dormant: /run does not exist and could not be created: {}",
                err
            ));
        }

        match fs::remove_file(SOCK_PATH) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(format!(
                    "dormant: failed to remove stale socket {}: {}",
                    SOCK_PATH, err
                ))
            }
        }

        let listener = UnixListener::bind(SOCK_PATH).map_err(|err| {
            format!(
                "dormant: failed to bind unix socket {}: {}",
                SOCK_PATH, err
            )
        })?;

        loop {
            let (mut stream, _) = listener.accept().map_err(|err| {
                format!(
                    "dormant: failed to accept on unix socket {}: {}",
                    SOCK_PATH, err
                )
            })?;

            let mut buf = [0u8; 1];
            stream.read_exact(&mut buf).map_err(|err| {
                format!(
                    "dormant: failed to read command byte from {}: {}",
                    SOCK_PATH, err
                )
            })?;

            match buf[0] {
                0x01 => {
                    self.state = SupervisorState::Downloading;
                    return Ok(());
                }
                0x02 => {
                    let response = self.build_slot_status_response()?;
                    stream.write_all(&response).map_err(|err| {
                        format!(
                            "dormant: failed to write status response on {}: {}",
                            SOCK_PATH, err
                        )
                    })?;
                    continue;
                }
                0xFF => {
                    eprintln!("stratsup: shutdown requested");
                    self.shutdown_requested = true;
                    return Ok(());
                }
                _ => {
                    continue;
                }
            }
        }
    }

    fn build_slot_status_response(&self) -> Result<[u8; 4], String> {
        let active_slot = efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT)
            .map_err(|err| format!("status query: failed to read STRAT_ACTIVE_SLOT: {}", err))?;
        let slot_a_status = efi_vars::read_u8(efi_vars::VAR_SLOT_A_STATUS)
            .map_err(|err| format!("status query: failed to read STRAT_SLOT_A_STATUS: {}", err))?;
        let slot_b_status = efi_vars::read_u8(efi_vars::VAR_SLOT_B_STATUS)
            .map_err(|err| format!("status query: failed to read STRAT_SLOT_B_STATUS: {}", err))?;
        let slot_c_status = efi_vars::read_u8(efi_vars::VAR_SLOT_C_STATUS)
            .map_err(|err| format!("status query: failed to read STRAT_SLOT_C_STATUS: {}", err))?;

        Ok([active_slot, slot_a_status, slot_b_status, slot_c_status])
    }

    fn handle_downloading(&mut self) -> Result<(), String> {
        let active_slot = efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT)
            .map_err(|err| format!("downloading: failed to read STRAT_ACTIVE_SLOT: {}", err))?;

        let pinned_slot_raw = efi_vars::read_u8(efi_vars::VAR_PINNED_SLOT)
            .map_err(|err| format!("downloading: failed to read STRAT_PINNED_SLOT: {}", err))?;
        let pinned_slot = if pinned_slot_raw <= SLOT_C {
            Some(pinned_slot_raw)
        } else {
            None
        };

        let target = Self::select_target_slot(active_slot, pinned_slot)?;

        let pinned_guard_before_write = efi_vars::slot_is_pinned(target.slot_id).map_err(|err| {
            format!(
                "downloading: failed pinned-slot guard check for slot {}: {}",
                target.slot_id, err
            )
        })?;
        if pinned_guard_before_write {
            return Err(format!(
                "downloading: refusing to write to pinned slot {}",
                target.slot_id
            ));
        }

        let update_config = Self::read_update_config()?;

        let image_bytes = Self::fetch_https_bytes(&update_config.image_url)?;
        let manifest_bytes = Self::fetch_https_bytes(&update_config.manifest_url)?;
        let signature_bytes = Self::fetch_https_bytes(&update_config.signature_url)?;

        Self::verify_manifest_signature(&manifest_bytes, &signature_bytes)?;
        Self::verify_image_sha256(&image_bytes, &manifest_bytes)?;

        let pinned_guard_pre_commit = efi_vars::slot_is_pinned(target.slot_id).map_err(|err| {
            format!(
                "downloading: failed pre-commit pinned-slot guard check for slot {}: {}",
                target.slot_id, err
            )
        })?;
        if pinned_guard_pre_commit {
            return Err(format!(
                "downloading: refusing to write to pinned slot {}",
                target.slot_id
            ));
        }

        Self::write_image_to_slot_device(target.block_device, &image_bytes)?;

        efi_vars::write_u8(target.status_var, STATUS_STAGING).map_err(|err| {
            format!(
                "downloading: failed to set slot {} status to staging via {}: {}",
                target.slot_id, target.status_var, err
            )
        })?;

        Self::emit_update_ready_notification()?;
        self.state = SupervisorState::ReadyToSwitch;
        Ok(())
    }

    fn handle_ready_to_switch(&mut self) -> Result<(), String> {
        Err("unimplemented: ready-to-switch state handler".to_string())
    }

    fn handle_pivoting(&mut self) -> Result<(), String> {
        Err("unimplemented: pivoting state handler".to_string())
    }

    fn handle_pivot_failed(&mut self) -> Result<(), String> {
        Err("unimplemented: pivot-failed state handler".to_string())
    }

    pub fn shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }

    fn select_target_slot(active_slot: u8, pinned_slot: Option<u8>) -> Result<SlotTarget, String> {
        if active_slot > SLOT_C {
            return Err(format!(
                "downloading: invalid active slot value {}",
                active_slot
            ));
        }

        for slot_id in [SLOT_C, SLOT_B, SLOT_A] {
            if slot_id == active_slot {
                continue;
            }
            if pinned_slot == Some(slot_id) {
                continue;
            }

            let status_var =
                efi_vars::slot_status_var(slot_id).map_err(|err| format!("downloading: {}", err))?;
            let block_device = Self::slot_block_device(slot_id)?;
            return Ok(SlotTarget {
                slot_id,
                status_var,
                block_device,
            });
        }

        Err("downloading: no writable slot available (active/pinned exclusions consumed all slots)"
            .to_string())
    }

    fn slot_block_device(slot_id: u8) -> Result<&'static str, String> {
        match slot_id {
            SLOT_A => Ok("/dev/sda2"),
            SLOT_B => Ok("/dev/sda3"),
            SLOT_C => Ok("/dev/sda4"),
            _ => Err(format!("downloading: invalid slot id {}", slot_id)),
        }
    }

    fn read_update_config() -> Result<UpdateConfig, String> {
        let contents = fs::read_to_string("/config/strat/update.conf").map_err(|err| {
            format!(
                "downloading: failed to read /config/strat/update.conf: {}",
                err
            )
        })?;

        let mut image_url: Option<String> = None;
        let mut manifest_url: Option<String> = None;
        let mut signature_url: Option<String> = None;

        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let Some((raw_key, raw_value)) = trimmed.split_once('=') else {
                continue;
            };

            let key = raw_key.trim();
            let value = raw_value.trim();

            match key {
                "image_url" => image_url = Some(value.to_string()),
                "manifest_url" => manifest_url = Some(value.to_string()),
                "signature_url" => signature_url = Some(value.to_string()),
                _ => {}
            }
        }

        let image_url = match image_url {
            Some(value) if !value.is_empty() => value,
            _ => return Err("downloading: update.conf missing required key: image_url".to_string()),
        };
        let manifest_url = match manifest_url {
            Some(value) if !value.is_empty() => value,
            _ => {
                return Err(
                    "downloading: update.conf missing required key: manifest_url".to_string(),
                )
            }
        };
        let signature_url = match signature_url {
            Some(value) if !value.is_empty() => value,
            _ => {
                return Err(
                    "downloading: update.conf missing required key: signature_url".to_string(),
                )
            }
        };

        if !image_url.starts_with("https://") {
            return Err("downloading: update.conf image_url must use https://".to_string());
        }
        if !manifest_url.starts_with("https://") {
            return Err("downloading: update.conf manifest_url must use https://".to_string());
        }
        if !signature_url.starts_with("https://") {
            return Err("downloading: update.conf signature_url must use https://".to_string());
        }

        Ok(UpdateConfig {
            image_url,
            manifest_url,
            signature_url,
        })
    }

    fn fetch_https_bytes(url: &str) -> Result<Vec<u8>, String> {
        if !url.starts_with("https://") {
            return Err(format!(
                "downloading: unsupported URL {} (only https:// is allowed)",
                url
            ));
        }

        let response = ureq::get(url)
            .call()
            .map_err(|err| format!("downloading: failed to fetch {}: {}", url, err))?;

        let mut body = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut body)
            .map_err(|err| {
                format!(
                    "downloading: failed to read response body from {}: {}",
                    url, err
                )
            })?;
        Ok(body)
    }

    fn verify_manifest_signature(manifest: &[u8], signature: &[u8]) -> Result<(), String> {
        if manifest.is_empty() {
            return Err("downloading: manifest is empty".to_string());
        }
        if signature.is_empty() {
            return Err("downloading: manifest signature is empty".to_string());
        }

        let pubkey_bytes = fs::read("/system/etc/strat/update-pubkey.asc").map_err(|err| {
            format!(
                "downloading: failed to read /system/etc/strat/update-pubkey.asc: {}",
                err
            )
        })?;
        let (pubkey, _) = SignedPublicKey::from_armor_single(Cursor::new(&pubkey_bytes))
            .map_err(|err| format!("downloading: failed to parse update public key: {}", err))?;
        let (sig, _) = StandaloneSignature::from_armor_single(Cursor::new(signature))
            .map_err(|err| format!("downloading: failed to parse manifest signature: {}", err))?;
        sig.verify(&pubkey, manifest).map_err(|err| {
            format!(
                "downloading: manifest signature verification failed: {}",
                err
            )
        })
    }

    fn verify_image_sha256(image: &[u8], manifest: &[u8]) -> Result<(), String> {
        let manifest_text = std::str::from_utf8(manifest)
            .map_err(|err| format!("downloading: manifest is not valid UTF-8: {}", err))?;
        let expected = Self::extract_sha256_from_manifest(manifest_text)?;

        let mut hasher = Sha256::new();
        hasher.update(image);
        let digest = hasher.finalize();
        let actual = Self::to_lower_hex(&digest);

        if actual == expected {
            Ok(())
        } else {
            Err(format!(
                "downloading: SHA256 mismatch (expected {}, got {})",
                expected, actual
            ))
        }
    }

    fn extract_sha256_from_manifest(manifest: &str) -> Result<String, String> {
        for token in manifest.split(|ch: char| {
            ch.is_whitespace() || matches!(ch, ':' | '=' | ',' | ';' | '(' | ')' | '[' | ']')
        }) {
            if token.len() != 64 {
                continue;
            }
            if token.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Ok(token.to_ascii_lowercase());
            }
        }
        Err("downloading: no SHA256 value found in manifest".to_string())
    }

    fn to_lower_hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            out.push(Self::hex_nibble((byte >> 4) & 0x0f));
            out.push(Self::hex_nibble(byte & 0x0f));
        }
        out
    }

    fn hex_nibble(value: u8) -> char {
        match value {
            0..=9 => (b'0' + value) as char,
            10..=15 => (b'a' + (value - 10)) as char,
            _ => '0',
        }
    }

    fn write_image_to_slot_device(device_path: &str, image_bytes: &[u8]) -> Result<(), String> {
        let path = Path::new(device_path);
        Self::ensure_not_symlink(path)?;

        let mut device = OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|err| format!("downloading: failed to open {}: {}", device_path, err))?;
        device
            .write_all(image_bytes)
            .map_err(|err| format!("downloading: failed to write image to {}: {}", device_path, err))?;
        device
            .sync_all()
            .map_err(|err| format!("downloading: failed to sync {}: {}", device_path, err))?;
        Ok(())
    }

    fn emit_update_ready_notification() -> Result<(), String> {
        let path = Path::new(NOTIFY_PATH);
        Self::ensure_not_symlink(path)?;
        fs::write(path, "Update downloaded. Restart when you're ready.")
            .map_err(|err| format!("downloading: failed to write {}: {}", NOTIFY_PATH, err))
    }

    fn ensure_not_symlink(path: &Path) -> Result<(), String> {
        match fs::symlink_metadata(path) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(format!(
                        "downloading: refusing to write through symlink {}",
                        path.display()
                    ));
                }
                Ok(())
            }
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!(
                "downloading: failed to inspect {}: {}",
                path.display(),
                err
            )),
        }
    }
}
