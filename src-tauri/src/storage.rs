//! Encrypted local storage and verified legacy migration. Per-scope SQLCipher
//! keys are randomly generated and Windows-protected; profile databases are
//! account-isolated, writes are bounded/atomic where required, and missing keys
//! never trigger plaintext fallback.

use rand::{rngs::OsRng, RngCore};
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::Manager;
use zeroize::Zeroizing;

pub const SCOPE: &str = if cfg!(debug_assertions) {
    "development-v2"
} else {
    "release-v2"
};
pub struct Storage {
    pub dir: PathBuf,
    key: Zeroizing<Vec<u8>>,
    _lock: File,
}
fn fail(_: impl std::fmt::Display) -> String {
    "Encrypted storage operation failed. Your existing data was kept; close older app versions and retry.".into()
}
/// Reject reparse/linked storage paths before trusting a local path as an owned
/// data store.
pub(crate) fn reject_link(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .map(|m| m.file_attributes() & 0x400 != 0)
        .unwrap_or(false)
    {
        return Err("Linked storage paths are not supported".into());
    }
    Ok(())
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(fail)?;
    f.write_all(bytes).map_err(fail)?;
    f.sync_all().map_err(fail)
}
fn key_text(key: &[u8]) -> Zeroizing<String> {
    use std::fmt::Write;
    let mut text = Zeroizing::new(String::with_capacity(67));
    text.push_str("x'");
    for value in key {
        write!(&mut *text, "{value:02x}").expect("writing a string cannot fail");
    }
    text.push('\'');
    text
}
fn connect(path: &Path, key: &[u8]) -> Result<Connection, String> {
    let c = Connection::open(path).map_err(fail)?;
    c.pragma_update(None, "key", key_text(key).as_str())
        .map_err(fail)?;
    let version: String = c
        .query_row("PRAGMA cipher_version", [], |r| r.get(0))
        .map_err(|_| "SQLCipher is required; unencrypted fallback is disabled".to_string())?;
    if version.is_empty() {
        return Err("SQLCipher is unavailable".into());
    }
    // SQLCipher defaults protect its key buffers. Do not toggle its optional
    // global SQLite allocator after connections have opened/closed: this
    // runtime combination recurses on the first encrypted write.
    c.pragma_update(None, "temp_store", "MEMORY")
        .map_err(fail)?;
    c.pragma_update(None, "secure_delete", "ON").map_err(fail)?;
    c.busy_timeout(Duration::from_secs(5)).map_err(fail)?;
    c.query_row("SELECT count(*) FROM sqlite_master",[],|r|r.get::<_,i64>(0)).map_err(|_|"Cannot unlock the encrypted library. Its key may be missing, damaged or belong to another Windows account. No data was reset.".to_string())?;
    Ok(c)
}
fn integrity(c: &Connection) -> Result<(), String> {
    let result: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(fail)?;
    if result != "ok" {
        return Err("Library integrity check failed; no data was removed".into());
    }
    Ok(())
}
fn counts(c: &Connection) -> Result<Vec<(String, i64)>, String> {
    let tables=c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").map_err(fail)?.query_map([],|r|r.get::<_,String>(0)).map_err(fail)?.collect::<Result<Vec<_>,_>>().map_err(fail)?;
    tables
        .into_iter()
        .map(|t| {
            let n = c
                .query_row(
                    &format!("SELECT count(*) FROM \"{}\"", t.replace('"', "\"\"")),
                    [],
                    |r| r.get(0),
                )
                .map_err(fail)?;
            Ok((t, n))
        })
        .collect()
}
fn hash(path: &Path) -> Result<Vec<u8>, String> {
    let mut f = File::open(path).map_err(fail)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 16384];
    loop {
        let n = f.read(&mut buf).map_err(fail)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_vec())
}
/// Publish only a verified encrypted export. A protected receipt makes cleanup
/// retryable after a crash; a changed legacy file is never silently discarded.
fn migrate(legacy: &Path, dir: &Path, key: &[u8]) -> Result<(), String> {
    let target = dir.join("library.sqlite");
    let staging = dir.join("migration.sqlite");
    let receipt = dir.join("migration.receipt");
    reject_link(legacy)?;
    if !target.exists() && legacy.exists() {
        if staging.exists() {
            fs::remove_file(&staging).map_err(fail)?;
        }
        // ATTACH inherits the source's no-create flags. This empty private
        // staging file receives encrypted pages only.
        write_new(&staging, &[])?;
        let source =
            Connection::open_with_flags(legacy, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(fail)?;
        source.busy_timeout(Duration::from_secs(2)).map_err(fail)?;
        source
            .pragma_update(None, "journal_mode", "DELETE")
            .map_err(fail)?;
        source
            .pragma_update(None, "temp_store", "MEMORY")
            .map_err(fail)?;
        integrity(&source)?;
        source
            .execute(
                "ATTACH DATABASE ?1 AS encrypted KEY ?2",
                rusqlite::params![staging.to_string_lossy(), key_text(key).as_str()],
            )
            .map_err(fail)?;
        source.execute_batch("BEGIN EXCLUSIVE").map_err(fail)?;
        let expected = counts(&source)?;
        let version: i64 = source
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(fail)?;
        source
            .query_row("SELECT sqlcipher_export('encrypted')", [], |_| Ok(()))
            .map_err(fail)?;
        source
            .execute_batch(&format!(
                "PRAGMA encrypted.user_version={version}; COMMIT; DETACH DATABASE encrypted;"
            ))
            .map_err(fail)?;
        source.close().map_err(|(_, e)| fail(e))?;
        let verified = connect(&staging, key)?;
        integrity(&verified)?;
        if counts(&verified)? != expected {
            return Err("Migration verification failed; original library retained".into());
        }
        verified.close().map_err(|(_, e)| fail(e))?;
        let digest = hash(legacy)?;
        if receipt.exists() {
            fs::remove_file(&receipt).map_err(fail)?;
        }
        write_new(&receipt, &crate::protection::protect(&digest)?)?;
        fs::rename(&staging, &target).map_err(fail)?;
    }
    if target.exists() && receipt.exists() {
        let c = connect(&target, key)?;
        integrity(&c)?;
        drop(c);
        if legacy.exists() {
            let expected = crate::protection::unprotect(&fs::read(&receipt).map_err(fail)?)?;
            if hash(legacy)?.as_slice() != expected.as_slice() {
                return Err("The old library changed during migration. Close older versions; both copies were kept for recovery.".into());
            }
            // Refuse deletion while an older Windows process has this database open.
            let exclusive = OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(legacy)
                .map_err(fail)?;
            drop(exclusive);
            fs::remove_file(legacy).map_err(fail)?;
        }
        // Retry sidecar cleanup even if a prior run removed the source and
        // stopped before it could remove the journal/receipt.
        for suffix in ["-wal", "-shm", "-journal"] {
            let side = PathBuf::from(format!("{}{suffix}", legacy.display()));
            reject_link(&side)?;
            if side.exists() {
                fs::remove_file(side).map_err(fail)?;
            }
        }
        fs::remove_file(receipt).map_err(fail)?;
    }
    Ok(())
}
impl Storage {
    /// Establish the exclusive store lock, protected key and supported
    /// migration state. Fail without replacing existing data when ownership,
    /// key or verification checks fail.
    pub fn initialize(root: &Path, import_legacy: bool) -> Result<Self, String> {
        reject_link(root)?;
        fs::create_dir_all(root).map_err(fail)?;
        reject_link(&root.join(format!("{SCOPE}.lock")))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .open(root.join(format!("{SCOPE}.lock")))
            .map_err(|_| {
                "This app profile is already open. Close the other copy first.".to_string()
            })?;
        let dir = root.join(SCOPE);
        reject_link(&dir)?;
        fs::create_dir_all(&dir).map_err(fail)?;
        crate::protection::restrict_directory(&dir)?;
        let key_path = dir.join("database.key");
        reject_link(&key_path)?;
        let key = if key_path.exists() {
            crate::protection::unprotect(&fs::read(&key_path).map_err(fail)?)?
        } else {
            if dir.join("library.sqlite").exists() || dir.join("migration.sqlite").exists() {
                return Err("Encrypted library key is missing. No new key was generated and no data was reset.".into());
            }
            let mut key = Zeroizing::new(vec![0u8; 32]);
            OsRng.fill_bytes(&mut key);
            write_new(&key_path, &crate::protection::protect(&key)?)?;
            key
        };
        if key.len() != 32 {
            return Err("Invalid encrypted library key; data retained".into());
        }
        if import_legacy {
            migrate(&root.join("library.sqlite"), &dir, &key)?;
        }
        let state = Self {
            dir,
            key,
            _lock: lock,
        };
        let c = state.open()?;
        integrity(&c)?;
        drop(c);
        Ok(state)
    }
    pub(crate) fn save_account_bytes(&self, raw: &[u8]) -> Result<(), String> {
        save_account_blob(&self.dir, raw)
    }
    pub(crate) fn load_account_bytes(&self) -> Result<Option<Zeroizing<Vec<u8>>>, String> {
        load_account_blob(&self.dir)
    }
    pub(crate) fn load_restorable_account(&self) -> Result<Option<Zeroizing<Vec<u8>>>, String> {
        if self.restore_blocked() {
            Ok(None)
        } else {
            self.load_account_bytes()
        }
    }
    pub fn open(&self) -> Result<Connection, String> {
        reject_link(&self.dir)?;
        reject_link(&self.dir.join("library.sqlite"))?;
        connect(&self.dir.join("library.sqlite"), &self.key)
    }
    /// Open the explicitly selected encrypted namespace; None refers to the
    /// shared settings/legacy store, not an inferred account.
    pub fn open_profile(&self, namespace: Option<&str>) -> Result<Connection, String> {
        let name = match namespace {
            Some(key) if key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()) => {
                format!("account-{key}.sqlite")
            }
            Some(_) => return Err("Invalid account storage context".into()),
            None => "settings.sqlite".into(),
        };
        reject_link(&self.dir)?;
        let path = self.dir.join(name);
        reject_link(&path)?;
        connect(&path, &self.key)
    }
    /// Persist a sign-out marker before removing credentials so a cleanup
    /// interruption cannot silently reconnect the account.
    pub fn block_account_restore(&self) -> Result<(), String> {
        let path = self.dir.join("signed-out");
        reject_link(&path)?;
        if !path.exists() {
            write_new(&path, b"1")?;
        }
        Ok(())
    }
    pub fn restore_blocked(&self) -> bool {
        // Any metadata error other than absence must fail closed.
        !matches!(fs::symlink_metadata(self.dir.join("signed-out")), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
    }
    /// Clear the durable sign-out block only after a protected account has been
    /// saved and verified.
    pub fn allow_account_restore(&self) -> Result<(), String> {
        let path = self.dir.join("signed-out");
        reject_link(&path)?;
        if path.exists() {
            fs::remove_file(path).map_err(fail)?;
        }
        Ok(())
    }
}
pub fn save_account(app: &tauri::AppHandle, raw: &[u8]) -> Result<(), String> {
    app.state::<Storage>().save_account_bytes(raw)
}
fn save_account_blob(dir: &Path, raw: &[u8]) -> Result<(), String> {
    let temp = dir.join("account.sealed.new");
    let target = dir.join("account.sealed");
    reject_link(&temp)?;
    reject_link(&target)?;
    if temp.exists() {
        fs::remove_file(&temp).map_err(fail)?;
    }
    write_new(&temp, &crate::protection::protect(raw)?)?;
    fs::rename(temp, target).map_err(|_| "Cannot save protected account".into())
}
pub fn load_account(app: &tauri::AppHandle) -> Result<Option<Zeroizing<Vec<u8>>>, String> {
    app.state::<Storage>().load_account_bytes()
}
fn load_account_blob(dir: &Path) -> Result<Option<Zeroizing<Vec<u8>>>, String> {
    let path = dir.join("account.sealed");
    reject_link(&path)?;
    if !path.exists() {
        return Ok(None);
    }
    if fs::metadata(&path).map_err(fail)?.len() > 1024 * 1024 {
        return Err("Protected account is too large".into());
    }
    crate::protection::unprotect(&fs::read(path).map_err(fail)?).map(Some)
}
/// Remove stored account credentials; encrypted per-account library/history
/// data is a separate retention concern.
pub fn forget_account(app: &tauri::AppHandle) -> Result<(), String> {
    let dir = data_dir(app)?;
    for name in ["account.sealed", "account.sealed.new"] {
        let path = dir.join(name);
        reject_link(&path)?;
        if path.exists() {
            fs::remove_file(path).map_err(|_| "Cannot remove protected account")?;
        }
    }
    Ok(())
}
pub fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app.state::<Storage>().dir.clone())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn account_databases_are_isolated_and_old_unassigned_data_is_not_imported() {
        let root = temp();
        let store = Storage::initialize(&root, false).unwrap();
        let a = "a".repeat(64);
        let b = "b".repeat(64);
        let legacy = store.open().unwrap();
        legacy.execute_batch("CREATE TABLE notes(text TEXT); INSERT INTO notes VALUES('unassigned legacy note');").unwrap();
        drop(legacy);
        for (key, note) in [(&a, "account A note"), (&b, "account B note")] {
            let c = store.open_profile(Some(key)).unwrap();
            c.execute_batch("CREATE TABLE notes(text TEXT);").unwrap();
            c.execute("INSERT INTO notes VALUES(?1)", [note]).unwrap();
        }
        // Returning to A preserves A's data, without B's or the legacy database.
        let c = store.open_profile(Some(&a)).unwrap();
        assert_eq!(
            c.query_row("SELECT group_concat(text) FROM notes", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "account A note"
        );
        drop(c);
        assert!(store.open_profile(Some("../library")).is_err());
        let bytes = fs::read(store.dir.join(format!("account-{a}.sqlite"))).unwrap();
        assert!(!bytes.windows(14).any(|b| b == b"account A note"));
        drop(store);
        let store = Storage::initialize(&root, false).unwrap();
        assert_eq!(
            store
                .open_profile(Some(&b))
                .unwrap()
                .query_row("SELECT text FROM notes", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "account B note"
        );
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn signed_out_marker_blocks_leftover_credentials_across_restart_until_fresh_save() {
        let root = temp();
        let store = Storage::initialize(&root, false).unwrap();
        store
            .save_account_bytes(b"synthetic old credential")
            .unwrap();
        store.block_account_restore().unwrap();
        // Simulate credential deletion failing: leave the sealed file in place.
        assert!(store.dir.join("account.sealed").exists());
        drop(store);
        let store = Storage::initialize(&root, false).unwrap();
        assert!(store.load_restorable_account().unwrap().is_none());
        store
            .save_account_bytes(b"synthetic fresh sign-in")
            .unwrap();
        assert!(store.load_restorable_account().unwrap().is_none());
        assert_eq!(
            store.load_account_bytes().unwrap().unwrap().as_slice(),
            b"synthetic fresh sign-in"
        );
        store.allow_account_restore().unwrap();
        assert_eq!(
            store.load_restorable_account().unwrap().unwrap().as_slice(),
            b"synthetic fresh sign-in"
        );
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
    fn temp() -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("audible-storage-test-{}", rand::random::<u64>()));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn encrypted_pages_wrong_keys_and_private_store() {
        let root = temp();
        let store = Storage::initialize(&root, false).unwrap();
        let c = store.open().unwrap();
        c.execute_batch(
            "CREATE TABLE notes(text TEXT); INSERT INTO notes VALUES('private dummy note');",
        )
        .unwrap();
        drop(c);
        let bytes = fs::read(store.dir.join("library.sqlite")).unwrap();
        assert!(!bytes.starts_with(b"SQLite format 3"));
        assert!(!bytes.windows(18).any(|b| b == b"private dummy note"));
        assert!(connect(&store.dir.join("library.sqlite"), &[2; 32]).is_err());
        assert!(Connection::open(store.dir.join("library.sqlite"))
            .unwrap()
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| r
                .get::<_, i64>(0))
            .is_err());
        assert_eq!(
            store
                .open()
                .unwrap()
                .query_row("SELECT text FROM notes", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "private dummy note"
        );
        assert!(Storage::initialize(&root, false).is_err());
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn verified_migration_preserves_notes_and_settings_and_removes_plaintext() {
        let root = temp();
        let legacy = root.join("library.sqlite");
        let c = Connection::open(&legacy).unwrap();
        c.execute_batch("CREATE TABLE notes(text TEXT); INSERT INTO notes VALUES('keep me'); CREATE TABLE preferences(key TEXT,value TEXT); INSERT INTO preferences VALUES('theme','ocean-dark');").unwrap();
        drop(c);
        let store = Storage::initialize(&root, true).unwrap();
        assert!(!legacy.exists());
        let c = store.open().unwrap();
        assert_eq!(
            c.query_row("SELECT text FROM notes", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "keep me"
        );
        drop(c);
        let dir = store.dir.clone();
        drop(store);
        let reopened = Storage::initialize(&root, true).unwrap();
        assert_eq!(reopened.dir, dir);
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_or_corrupt_key_never_resets_data() {
        let root = temp();
        let store = Storage::initialize(&root, false).unwrap();
        store
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TABLE notes(text TEXT); INSERT INTO notes VALUES('preserve this');",
            )
            .unwrap();
        let path = store.dir.join("database.key");
        let db = store.dir.join("library.sqlite");
        drop(store);
        let before = fs::read(&db).unwrap();
        fs::write(&path, b"corrupt").unwrap();
        assert!(Storage::initialize(&root, false).is_err());
        assert_eq!(fs::read(&db).unwrap(), before);
        fs::remove_file(path).unwrap();
        assert!(Storage::initialize(&root, false).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn protected_accounts_support_large_tokens_and_atomic_replacement() {
        let root = temp();
        let store = Storage::initialize(&root, false).unwrap();
        let dummy = vec![b'Q'; 16384];
        save_account_blob(&store.dir, &dummy).unwrap();
        let bytes = fs::read(store.dir.join("account.sealed")).unwrap();
        assert!(!bytes.windows(64).any(|v| v == &dummy[..64]));
        assert_eq!(
            load_account_blob(&store.dir).unwrap().unwrap().as_slice(),
            dummy.as_slice()
        );
        save_account_blob(&store.dir, b"replacement dummy token").unwrap();
        assert_eq!(
            load_account_blob(&store.dir).unwrap().unwrap().as_slice(),
            b"replacement dummy token"
        );
        assert!(!store.dir.join("account.sealed.new").exists());
        fs::write(store.dir.join("account.sealed"), b"tampered").unwrap();
        assert!(load_account_blob(&store.dir).is_err());
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn changed_legacy_data_is_never_discarded_during_recovery() {
        let root = temp();
        let store = Storage::initialize(&root, false).unwrap();
        let target = store.dir.join("library.sqlite");
        store
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TABLE notes(text TEXT); INSERT INTO notes VALUES('encrypted copy');",
            )
            .unwrap();
        let legacy = root.join("library.sqlite");
        let c = Connection::open(&legacy).unwrap();
        c.execute_batch("CREATE TABLE notes(text TEXT); INSERT INTO notes VALUES('original');")
            .unwrap();
        drop(c);
        write_new(
            &store.dir.join("migration.receipt"),
            &crate::protection::protect(&hash(&legacy).unwrap()).unwrap(),
        )
        .unwrap();
        Connection::open(&legacy)
            .unwrap()
            .execute("INSERT INTO notes VALUES('newer data')", [])
            .unwrap();
        assert!(migrate(&legacy, &store.dir, &store.key).is_err());
        assert!(legacy.exists());
        assert!(target.exists());
        assert!(store.dir.join("migration.receipt").exists());
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn interrupted_cleanup_removes_plaintext_sidecars_on_retry() {
        let root = temp();
        let store = Storage::initialize(&root, false).unwrap();
        store
            .open()
            .unwrap()
            .execute_batch("CREATE TABLE notes(text TEXT); INSERT INTO notes VALUES('keep');")
            .unwrap();
        write_new(
            &store.dir.join("migration.receipt"),
            &crate::protection::protect(&[5; 32]).unwrap(),
        )
        .unwrap();
        fs::write(root.join("library.sqlite-wal"), b"old dummy journal").unwrap();
        migrate(&root.join("library.sqlite"), &store.dir, &store.key).unwrap();
        assert!(!root.join("library.sqlite-wal").exists());
        assert!(!store.dir.join("migration.receipt").exists());
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
}
