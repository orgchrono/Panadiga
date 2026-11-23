#![deny(unsafe_code)]

//! Multi-process advisory Vault locking (`.vault.lock`) and parent-directory `fsync` helpers.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Seek, Write};
use std::path::Path;

use crate::journal::JournalError;

pub const VAULT_LOCK_FILENAME: &str = ".vault.lock";

/// Lock mode acquired on `vault_dir/.vault.lock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultLockMode {
    Shared,
    Exclusive,
}

/// RAII handle holding a shared or exclusive OS advisory lock on `.vault.lock`.
#[derive(Debug)]
pub struct VaultLock {
    _file: File,
    mode: VaultLockMode,
}

fn parse_lock_owner_metadata(lock_path: &Path) -> (Option<u32>, Option<String>) {
    let Ok(raw) = std::fs::read_to_string(lock_path) else {
        return (None, None);
    };
    let mut pid = None;
    let mut actor = None;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("pid=") {
            pid = v.trim().parse::<u32>().ok();
        } else if let Some(v) = line.strip_prefix("actor=") {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                actor = Some(trimmed.to_string());
            }
        }
    }
    (pid, actor)
}

impl VaultLock {
    /// Non-blocking acquire (`Exclusive` for mutating vault/journal operations, `Shared` for reads).
    pub fn try_acquire(
        vault_dir: &Path,
        mode: VaultLockMode,
        actor_label: &str,
    ) -> Result<Self, JournalError> {
        std::fs::create_dir_all(vault_dir)?;
        let lock_path = vault_dir.join(VAULT_LOCK_FILENAME);
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;

        let lock_res = match mode {
            VaultLockMode::Exclusive => file.try_lock(),
            VaultLockMode::Shared => file.try_lock_shared(),
        };
        match lock_res {
            Ok(()) => {
                if mode == VaultLockMode::Exclusive {
                    let _ = file.set_len(0);
                    let _ = file.rewind();
                    let _ = writeln!(file, "pid={}\nactor={}", std::process::id(), actor_label);
                    let _ = file.flush();
                }
                Ok(Self { _file: file, mode })
            }
            Err(TryLockError::WouldBlock) => {
                let (pid, actor) = parse_lock_owner_metadata(&lock_path);
                Err(JournalError::VaultLocked { pid, actor })
            }
            Err(TryLockError::Error(io_err)) => Err(JournalError::Io(io_err)),
        }
    }

    /// Returns whether this handle holds a shared or exclusive lock.
    #[must_use]
    pub const fn mode(&self) -> VaultLockMode {
        self.mode
    }
}

/// Flushes parent directory metadata after creating/renaming durable vault files on Unix.
pub fn sync_parent_dir(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        File::open(parent)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
