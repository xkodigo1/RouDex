//! Where RouDex keeps its files: `$ROUDEX_HOME`, by default `~/.roudex`, on
//! every system (spec 005, D1). Codex does the same with `~/.codex`.
//!
//! On Windows, versions before 0.9.0 kept `router.toml` and the store in
//! `%LOCALAPPDATA%\RouDex`; [`migrate_legacy`] copies them once.

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::path::{Path, PathBuf};

const STORE_FILE: &str = "router.sqlite3";
const CONFIG_FILE: &str = "router.toml";

/// `$ROUDEX_HOME`, else `.roudex` in the user's home folder.
pub fn home() -> PathBuf {
    if let Some(home) = std::env::var_os("ROUDEX_HOME").filter(|home| !home.is_empty()) {
        return PathBuf::from(home);
    }
    let variables = if cfg!(windows) {
        ["USERPROFILE", "HOME"]
    } else {
        ["HOME", "USERPROFILE"]
    };
    variables
        .into_iter()
        .find_map(|name| std::env::var_os(name).filter(|value| !value.is_empty()))
        .map(|home| PathBuf::from(home).join(".roudex"))
        .unwrap_or_else(|| PathBuf::from(".roudex"))
}

/// The local store: threads, turns, specs, tasks and runs.
pub fn store_path() -> PathBuf {
    home().join(STORE_FILE)
}

/// Tier targets and the other settings.
pub fn config_path() -> PathBuf {
    home().join(CONFIG_FILE)
}

/// Where versions before 0.9.0 kept their data: `%LOCALAPPDATA%\RouDex` on
/// Windows. Other systems had no fixed location to migrate from.
pub fn legacy_dir() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    std::env::var_os("LOCALAPPDATA")
        .filter(|root| !root.is_empty())
        .map(|root| PathBuf::from(root).join("RouDex"))
}

/// What a migration copied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Migration {
    pub from: PathBuf,
    pub to: PathBuf,
    pub store: bool,
    pub config: bool,
}

/// Copy the pre-0.9.0 data into [`home`] once. See [`migrate_from`].
pub fn migrate_legacy() -> Result<Option<Migration>, String> {
    match legacy_dir() {
        Some(legacy) => migrate_from(&legacy, &home()),
        None => Ok(None),
    }
}

/// Copy `legacy`'s store and `router.toml` into `home` when `home` has no
/// store yet. The originals are never changed, and a file that already
/// exists in `home` is never overwritten.
pub fn migrate_from(legacy: &Path, home: &Path) -> Result<Option<Migration>, String> {
    let source = legacy.join(STORE_FILE);
    let target = home.join(STORE_FILE);
    if target.exists() || !source.is_file() {
        return Ok(None);
    }
    std::fs::create_dir_all(home)
        .map_err(|error| format!("could not create {}: {error}", home.display()))?;
    copy_store(&source, &target)?;
    let config_source = legacy.join(CONFIG_FILE);
    let config_target = home.join(CONFIG_FILE);
    let config = config_source.is_file() && !config_target.exists();
    if config {
        std::fs::copy(&config_source, &config_target).map_err(|error| {
            format!(
                "copied the store but not {}: {error}",
                config_source.display()
            )
        })?;
    }
    Ok(Some(Migration {
        from: legacy.to_path_buf(),
        to: home.to_path_buf(),
        store: true,
        config,
    }))
}

/// A consistent copy, even with a write-ahead log beside the source, written
/// under a temporary name and renamed so a crash never leaves half a store.
fn copy_store(source: &Path, target: &Path) -> Result<(), String> {
    let partial = target.with_extension("sqlite3.partial");
    let _ = std::fs::remove_file(&partial);
    let copy = || -> rusqlite::Result<()> {
        let connection = Connection::open_with_flags(
            source,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.execute("VACUUM INTO ?1", [partial.to_string_lossy()])?;
        Ok(())
    };
    copy().map_err(|error| format!("could not copy {}: {error}", source.display()))?;
    std::fs::rename(&partial, target).map_err(|error| {
        let _ = std::fs::remove_file(&partial);
        format!("could not move the copy to {}: {error}", target.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    struct Folder(PathBuf);

    impl Folder {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("roudex-paths-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A legacy store in WAL mode with a row still in the log.
    fn legacy_store(dir: &Path) -> Connection {
        std::fs::create_dir_all(dir).unwrap();
        let connection = Connection::open(dir.join(STORE_FILE)).unwrap();
        connection
            .pragma_update(None, "journal_mode", "wal")
            .unwrap();
        connection
            .execute_batch("PRAGMA wal_autocheckpoint = 0; CREATE TABLE notes (text TEXT);")
            .unwrap();
        connection
            .execute("INSERT INTO notes VALUES (?1)", params!["kept"])
            .unwrap();
        std::fs::write(dir.join(CONFIG_FILE), "version = 1\n").unwrap();
        connection
    }

    fn modified(path: &Path) -> std::time::SystemTime {
        path.metadata().unwrap().modified().unwrap()
    }

    #[test]
    fn the_first_start_copies_the_store_and_the_config_once() {
        let root = Folder::new();
        let legacy = root.0.join("LocalAppData").join("RouDex");
        let home = root.0.join(".roudex");
        let _open = legacy_store(&legacy);
        let before = (
            modified(&legacy.join(STORE_FILE)),
            std::fs::read(legacy.join(CONFIG_FILE)).unwrap(),
        );

        let migration = migrate_from(&legacy, &home).unwrap().expect("a migration");
        assert!(migration.store && migration.config);
        let copy = Connection::open(home.join(STORE_FILE)).unwrap();
        let text: String = copy
            .query_row("SELECT text FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(text, "kept", "rows still in the WAL are copied");
        assert_eq!(
            std::fs::read_to_string(home.join(CONFIG_FILE)).unwrap(),
            "version = 1\n"
        );
        assert_eq!(
            (
                modified(&legacy.join(STORE_FILE)),
                std::fs::read(legacy.join(CONFIG_FILE)).unwrap()
            ),
            before,
            "the originals stay as they were"
        );
        assert!(!home.join("router.sqlite3.partial").exists());

        assert_eq!(migrate_from(&legacy, &home).unwrap(), None, "only once");
    }

    #[test]
    fn existing_data_in_home_is_never_overwritten() {
        let root = Folder::new();
        let legacy = root.0.join("legacy");
        let home = root.0.join("home");
        let _open = legacy_store(&legacy);
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(STORE_FILE), b"mine").unwrap();
        assert_eq!(migrate_from(&legacy, &home).unwrap(), None);
        assert_eq!(std::fs::read(home.join(STORE_FILE)).unwrap(), b"mine");

        // A config of its own is kept even when the store is copied.
        let other = root.0.join("other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join(CONFIG_FILE), "version = 1\n# mine\n").unwrap();
        let migration = migrate_from(&legacy, &other).unwrap().unwrap();
        assert!(migration.store && !migration.config);
        assert_eq!(
            std::fs::read_to_string(other.join(CONFIG_FILE)).unwrap(),
            "version = 1\n# mine\n"
        );
    }

    #[test]
    fn nothing_to_migrate_without_a_legacy_store() {
        let root = Folder::new();
        assert_eq!(
            migrate_from(&root.0.join("missing"), &root.0.join("home")).unwrap(),
            None
        );
        assert!(!root.0.join("home").exists(), "no empty folder is created");
    }
}
