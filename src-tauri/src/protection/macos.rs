//! AES-256-GCM envelopes with a random wrapping key held in macOS Keychain.
//! Keychain denial/missing keys stop recovery; existing data is never re-keyed.
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Mutex};
use zeroize::Zeroizing;

const HEADER: &[u8] = b"LLMAC01";
const LIMIT: usize = 1024 * 1024;
static KEY_LOCK: Mutex<()> = Mutex::new(());
const SERVICE: &str = if cfg!(test) {
    "dev.noslavde.listeners-library.macos.tests"
} else if cfg!(debug_assertions) {
    "dev.noslavde.listeners-library.macos-alpha.development"
} else {
    "dev.noslavde.listeners-library.macos-alpha.release"
};
fn failure() -> String {
    "macOS could not unlock protected data. Unlock your Keychain and retry; do not delete its key or the library.".into()
}
fn wrapping_key(create: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    let _guard = KEY_LOCK.lock().map_err(|_| failure())?;
    let entry = keyring::Entry::new(SERVICE, "wrapping-key-v1").map_err(|_| failure())?;
    stored_key(&entry, create)
}
fn stored_key(entry: &keyring::Entry, create: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    let key = match entry.get_secret() {
        Ok(key) => Zeroizing::new(key),
        Err(keyring::Error::NoEntry) if create => {
            let mut key = Zeroizing::new(vec![0; 32]);
            OsRng.fill_bytes(&mut key);
            entry.set_secret(&key).map_err(|_| failure())?;
            // Verify persistence before any dependent data can be published.
            if entry
                .get_secret()
                .map(Zeroizing::new)
                .map_err(|_| failure())?
                .as_slice()
                != key.as_slice()
            {
                return Err(failure());
            }
            key
        }
        Err(_) => return Err(failure()),
    };
    if key.len() != 32 {
        return Err(failure());
    }
    Ok(key)
}
fn seal(key: &[u8], input: &[u8]) -> Result<Vec<u8>, String> {
    if input.is_empty() || input.len() > LIMIT {
        return Err("Invalid protected storage size".into());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| failure())?;
    let mut nonce = [0; 12];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: input,
                aad: SERVICE.as_bytes(),
            },
        )
        .map_err(|_| failure())?;
    Ok([HEADER, &nonce, &encrypted].concat())
}
fn unseal(key: &[u8], input: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    let offset = HEADER.len() + 12;
    if !input.starts_with(HEADER) || input.len() <= offset + 16 || input.len() > LIMIT + offset + 16
    {
        return Err(failure());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| failure())?;
    cipher
        .decrypt(
            Nonce::from_slice(&input[HEADER.len()..offset]),
            Payload {
                msg: &input[offset..],
                aad: SERVICE.as_bytes(),
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| failure())
}
pub fn protect(input: &[u8]) -> Result<Vec<u8>, String> {
    let key = wrapping_key(false)?;
    seal(key.as_slice(), input)
}
pub fn initialize(allow_create: bool) -> Result<(), String> {
    wrapping_key(allow_create).map(|_| ())
}
pub fn forget_wrapping_key() -> Result<(), String> {
    let _guard = KEY_LOCK.lock().map_err(|_| failure())?;
    match keyring::Entry::new(SERVICE, "wrapping-key-v1")
        .map_err(|_| failure())?
        .delete_credential()
    {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(failure()),
    }
}
pub fn unprotect(input: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    let key = wrapping_key(false)?;
    unseal(key.as_slice(), input)
}
pub fn restrict_directory(path: &Path) -> Result<(), String> {
    crate::storage::reject_link(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| failure())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelopes_reject_wrong_keys_tampering_and_truncation() {
        let secret = b"synthetic account data";
        let key = [42; 32];
        let mut blob = seal(&key, secret).unwrap();
        assert!(!blob.windows(secret.len()).any(|w| w == secret));
        assert_eq!(unseal(&key, &blob).unwrap().as_slice(), secret);
        assert_ne!(blob, seal(&key, secret).unwrap());
        assert!(unseal(&[43; 32], &blob).is_err());
        for n in 0..blob.len() {
            assert!(unseal(&key, &blob[..n]).is_err());
        }
        let last = blob.len() - 1;
        blob[last] ^= 1;
        assert!(unseal(&key, &blob).is_err());
    }
    #[test]
    fn native_keychain_roundtrip() {
        initialize(true).unwrap();
        let data = b"synthetic native Keychain test";
        let blob = protect(data).unwrap();
        assert_eq!(unprotect(&blob).unwrap().as_slice(), data);
    }
    #[test]
    fn missing_keychain_key_is_not_replaced_on_recovery() {
        let service = format!(
            "dev.noslavde.listeners-library.macos-test-{}",
            rand::random::<u64>()
        );
        let entry = keyring::Entry::new(&service, "synthetic-key").unwrap();
        assert!(stored_key(&entry, false).is_err());
        let key = stored_key(&entry, true).unwrap();
        let blob = seal(&key, b"synthetic preserved data").unwrap();
        entry.delete_credential().unwrap();
        assert!(stored_key(&entry, false).is_err());
        assert!(matches!(entry.get_secret(), Err(keyring::Error::NoEntry)));
        assert_eq!(
            unseal(&key, &blob).unwrap().as_slice(),
            b"synthetic preserved data"
        );
    }
}
