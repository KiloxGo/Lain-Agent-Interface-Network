//! Owner-only native local files shared by the CLI and device host.
use anyhow::{Context, Result, ensure};
#[cfg(windows)]
mod windows_security;
use std::{fs, io::Write, path::Path};

fn reject_link(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).context("inspect local settings")?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "local settings cannot be symbolic links"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "local settings cannot be reparse points"
        );
    }
    Ok(())
}
#[cfg(unix)]
pub fn private(path: &Path, directory: bool, set: bool) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    reject_link(path)?;
    let metadata = fs::metadata(path)?;
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "local settings must belong to the current owner"
    );
    ensure!(
        metadata.is_dir() == directory,
        "unexpected local settings file type"
    );
    if !directory {
        ensure!(
            metadata.is_file() && metadata.nlink() == 1,
            "credentials must be ordinary, unlinked files"
        );
    }
    let mode = if directory { 0o700 } else { 0o600 };
    if set {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    ensure!(
        fs::metadata(path)?.permissions().mode() & 0o777 == mode,
        "local settings permissions are not private; restore 0700 directories and 0600 files"
    );
    Ok(())
}
#[cfg(windows)]
pub fn private(path: &Path, directory: bool, set: bool) -> Result<()> {
    reject_link(path)?;
    windows_security::private(path, directory, set)
}
pub fn create_private(path: &Path, content: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(path)
        .context("create local file without overwriting")?;
    // Parent is already private; Windows child inherits its owner-only ACL.
    private(path, false, true)?;
    file.write_all(content)?;
    file.sync_all()?;
    Ok(())
}
pub fn outside_checkout(path: &Path) -> Result<()> {
    let canonical = path
        .canonicalize()
        .context("resolve local configuration directory")?;
    ensure!(
        !canonical.ancestors().any(|p| p.join(".git").exists()),
        "local credentials must be outside a Git checkout"
    );
    Ok(())
}

/// Replace one private owned file from an exclusively created same-directory
/// stage. Existing files with unsafe ownership/link/ACL state are never repaired.
pub fn atomic_private(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent().context("local file parent missing")?;
    private(parent, true, false)?;
    outside_checkout(parent)?;
    if fs::symlink_metadata(path).is_ok() {
        private(path, false, false)?;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let mut stage = None;
    for attempt in 0..16 {
        let name = parent.join(format!(
            ".lain-cache-stage-{}-{stamp}-{attempt}",
            std::process::id()
        ));
        match create_private(&name, content) {
            Ok(()) => {
                stage = Some(name);
                break;
            }
            Err(e)
                if e.chain().any(|e| {
                    e.downcast_ref::<std::io::Error>()
                        .is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists)
                }) =>
            {
                continue;
            }
            Err(e) => {
                if private(&name, false, false).is_ok() {
                    let _ = fs::remove_file(&name);
                }
                return Err(e);
            }
        }
    }
    let stage = stage.context("cannot allocate private cache stage")?;
    let result = (|| {
        if fs::symlink_metadata(path).is_ok() {
            private(path, false, false)?;
        }
        fs::rename(&stage, path).context("atomically replace private file")?;
        private(path, false, false)?;
        #[cfg(unix)]
        {
            fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    })();
    if stage.exists() {
        let _ = fs::remove_file(&stage);
    }
    result
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn atomic_private_replaces_owned_cache_and_keeps_only_one_private_file() {
        let dir = tempfile::tempdir().unwrap();
        private(dir.path(), true, true).unwrap();
        let path = dir.path().join("cache.json");
        atomic_private(&path, b"first").unwrap();
        atomic_private(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        private(&path, false, false).unwrap();
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[cfg(unix)]
    #[test]
    fn atomic_private_rejects_links_and_permissive_cache_without_repair() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = tempfile::tempdir().unwrap();
        private(dir.path(), true, true).unwrap();
        let path = dir.path().join("cache.json");
        create_private(&path, b"original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(atomic_private(&path, b"new").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        fs::remove_file(&path).unwrap();
        symlink(dir.path().join("outside"), &path).unwrap();
        assert!(atomic_private(&path, b"new").is_err());
        assert!(!dir.path().join("outside").exists());
    }
}
