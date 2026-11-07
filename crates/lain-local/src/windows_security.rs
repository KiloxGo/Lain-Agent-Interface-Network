//! Handle-based owner-only ACLs. No shell/process survives a crashed caller.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::c_void,
    mem::{offset_of, size_of},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE, LocalFree},
    Security::{Authorization::*, *},
    Storage::FileSystem::*,
    System::{
        SystemServices::ACCESS_ALLOWED_ACE_TYPE,
        Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

fn code(status: u32) -> std::io::Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(status as i32))
    }
}
fn ok(value: i32) -> std::io::Result<()> {
    if value != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: allocation returned by GetSecurityInfo/ConvertStringSidToSidW.
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}

struct CurrentUser {
    _token: OwnedHandle,
    info: Vec<usize>,
}
impl CurrentUser {
    fn load() -> Result<Self> {
        let mut token = null_mut();
        // SAFETY: current-process pseudo handle is borrowed; output token is owned.
        unsafe {
            ok(OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY,
                &mut token,
            ))
        }
        .context("open current owner token")?;
        // SAFETY: successful OpenProcessToken returned an owned handle.
        let token = unsafe { OwnedHandle::from_raw_handle(token) };
        let mut needed = 0;
        // SAFETY: zero-size sizing query, no output buffer supplied.
        unsafe {
            GetTokenInformation(token.as_raw_handle(), TokenUser, null_mut(), 0, &mut needed);
        }
        ensure!(
            (size_of::<TOKEN_USER>()..=65536).contains(&(needed as usize)),
            "invalid owner token size"
        );
        // usize allocation preserves TOKEN_USER alignment, unlike casting a byte buffer.
        let mut info = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
        // SAFETY: aligned buffer has at least needed writable bytes and remains owned.
        unsafe {
            ok(GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                info.as_mut_ptr().cast(),
                needed,
                &mut needed,
            ))
        }
        .context("read current owner SID")?;
        let user = Self {
            _token: token,
            info,
        };
        // SAFETY: TOKEN_USER was fully written by Windows; backing allocation stays alive.
        ensure!(
            unsafe { IsValidSid(user.sid()) } != 0,
            "invalid current owner SID"
        );
        Ok(user)
    }
    fn sid(&self) -> PSID {
        // SAFETY: aligned TOKEN_USER is in self.info; SID points into the same stable buffer.
        unsafe { (*self.info.as_ptr().cast::<TOKEN_USER>()).User.Sid }
    }
}

fn native_path(path: &Path) -> Result<Vec<u16>> {
    ensure!(
        !path.as_os_str().encode_wide().any(|unit| unit == 0),
        "local settings path contains NUL"
    );
    // Lexical Windows normalization, never canonicalize: resolving a reparse
    // point here would hide it from the handle-based checks in open().
    let absolute = std::path::absolute(path).context("resolve local settings path")?;
    let mut wide = absolute.as_os_str().encode_wide().collect::<Vec<_>>();
    // Match ordinary Win32/std file semantics for short paths. Long paths need
    // an explicit namespace even when the process/system is not longPathAware.
    // absolute() has already resolved relative components and '/' separators;
    // existing verbatim/device paths retain their original namespace.
    if wide.len() >= 260
        && !wide.starts_with(&[92, 92, 63, 92])
        && !wide.starts_with(&[92, 92, 46, 92])
    {
        wide = if wide.starts_with(&[92, 92]) {
            r"\\?\UNC\"
                .encode_utf16()
                .chain(wide.into_iter().skip(2))
                .collect()
        } else {
            r"\\?\".encode_utf16().chain(wide).collect()
        };
    }
    wide.push(0);
    Ok(wide)
}

fn open(path: &Path, directory: bool, set: bool) -> Result<OwnedHandle> {
    let wide = native_path(path)?;
    let access = READ_CONTROL | if set { WRITE_DAC | WRITE_OWNER } else { 0 };
    // SAFETY: terminated path, no inherited handle/security attributes; open existing only.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    ensure!(
        handle != INVALID_HANDLE_VALUE,
        "open local settings ACL: {}",
        std::io::Error::last_os_error()
    );
    // SAFETY: successful CreateFileW returned an exclusively owned handle.
    let file = unsafe { OwnedHandle::from_raw_handle(handle) };
    let mut metadata = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: valid handle and writable native metadata structure.
    unsafe {
        ok(GetFileInformationByHandle(
            file.as_raw_handle(),
            &mut metadata,
        ))
    }
    .context("inspect opened private object")?;
    ensure!(
        metadata.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT == 0,
        "local settings cannot be reparse points"
    );
    ensure!(
        (metadata.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) == directory,
        "unexpected local settings file type"
    );
    ensure!(
        directory || metadata.nNumberOfLinks == 1,
        "credentials must be ordinary, unlinked files"
    );
    Ok(file)
}
fn owner_acl(sid: PSID, directory: bool) -> Result<Vec<usize>> {
    // SAFETY: caller retains the valid current-user token buffer for this operation.
    let length = unsafe { GetLengthSid(sid) } as usize;
    let bytes = size_of::<ACL>() + offset_of!(ACCESS_ALLOWED_ACE, SidStart) + length;
    let mut buffer = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let acl = buffer.as_mut_ptr().cast::<ACL>();
    // SAFETY: aligned ACL buffer is large enough for its header and one complete SID ACE.
    unsafe {
        ok(InitializeAcl(acl, bytes as u32, ACL_REVISION))?;
        ok(AddAccessAllowedAceEx(
            acl,
            ACL_REVISION,
            if directory {
                CONTAINER_INHERIT_ACE | OBJECT_INHERIT_ACE
            } else {
                0
            },
            FILE_ALL_ACCESS,
            sid,
        ))?;
    }
    Ok(buffer)
}
fn set_owner(file: HANDLE, user: &CurrentUser, directory: bool) -> Result<()> {
    let mut acl = owner_acl(user.sid(), directory)?;
    // SAFETY: retained handle has WRITE_DAC/WRITE_OWNER; SID and ACL buffers remain alive.
    unsafe {
        code(SetSecurityInfo(
            file,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION
                | DACL_SECURITY_INFORMATION
                | PROTECTED_DACL_SECURITY_INFORMATION,
            user.sid(),
            null_mut(),
            acl.as_mut_ptr().cast(),
            null_mut(),
        ))
    }
    .context("protect new local object for current owner")
}
fn validate(file: HANDLE, user: &CurrentUser) -> Result<()> {
    let (mut owner, mut acl, mut descriptor) = (null_mut(), null_mut(), null_mut());
    // SAFETY: owned file handle has READ_CONTROL; output pointers are initialized.
    unsafe {
        code(GetSecurityInfo(
            file,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            &mut acl,
            null_mut(),
            &mut descriptor,
        ))
    }
    .context("read local owner/DACL")?;
    let allocation = LocalAllocation(descriptor);
    let (mut control, mut revision) = (0, 0);
    // SAFETY: valid descriptor owns both owner and ACL; current SID buffer is retained.
    unsafe {
        ok(GetSecurityDescriptorControl(
            allocation.0,
            &mut control,
            &mut revision,
        ))?;
        ensure!(
            !owner.is_null() && IsValidSid(owner) != 0 && EqualSid(owner, user.sid()) != 0,
            "local settings must belong to the current owner"
        );
        ensure!(
            control & SE_DACL_PROTECTED != 0,
            "local settings ACL inheritance must be disabled"
        );
        ensure!(
            !acl.is_null() && IsValidAcl(acl) != 0 && (*acl).AceCount > 0,
            "local settings require a nonempty owner-only ACL"
        );
        for index in 0..u32::from((*acl).AceCount) {
            let mut raw = null_mut();
            ok(GetAce(acl, index, &mut raw))?;
            let header = &*raw.cast::<ACE_HEADER>();
            ensure!(
                u32::from(header.AceType) == ACCESS_ALLOWED_ACE_TYPE
                    && usize::from(header.AceSize) >= offset_of!(ACCESS_ALLOWED_ACE, SidStart) + 8,
                "local settings ACL must contain only direct owner allow entries"
            );
            let ace = raw.cast::<ACCESS_ALLOWED_ACE>();
            let sid = std::ptr::addr_of!((*ace).SidStart)
                .cast_mut()
                .cast::<c_void>();
            ensure!(
                IsValidSid(sid) != 0
                    && usize::from(header.AceSize)
                        >= offset_of!(ACCESS_ALLOWED_ACE, SidStart) + GetLengthSid(sid) as usize
                    && EqualSid(sid, user.sid()) != 0,
                "local settings ACL must grant only the current owner"
            );
        }
    }
    Ok(())
}
pub(super) fn private(path: &Path, directory: bool, set: bool) -> Result<()> {
    let user = CurrentUser::load()?;
    let file = open(path, directory, set)?;
    if set {
        set_owner(file.as_raw_handle(), &user, directory)?;
    }
    validate(file.as_raw_handle(), &user)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_paths_preserve_namespaces_and_reject_nul_without_filesystem_resolution() {
        fn decoded(path: &str) -> String {
            let wide = native_path(Path::new(path)).unwrap();
            assert_eq!(wide.last(), Some(&0));
            String::from_utf16(&wide[..wide.len() - 1]).unwrap()
        }
        let tail = format!("{}/{}", "a".repeat(140), "b".repeat(140));
        assert_eq!(
            decoded(&format!("C:/missing/../{tail}")),
            format!("\\\\?\\C:\\{}", tail.replace('/', "\\"))
        );
        assert_eq!(
            decoded(&format!("//unreachable/share/{tail}")),
            format!(
                "\\\\?\\UNC\\unreachable\\share\\{}",
                tail.replace('/', "\\")
            )
        );
        let verbatim = format!(r"\\?\C:\{tail}");
        assert_eq!(decoded(&verbatim), verbatim);
        assert_eq!(decoded("C:/missing/../file"), r"C:\file");
        assert!(native_path(Path::new("C:\\invalid\0name")).is_err());
    }

    #[test]
    fn native_private_acl_supports_long_unicode_paths_and_atomic_cache_replace() {
        let root = tempfile::tempdir().unwrap();
        super::private(root.path(), true, true).unwrap();
        let mut directory = root.path().to_path_buf();
        for index in 0..5 {
            directory.push(format!("layer-{index}-{}", "目录".repeat(24)));
        }
        std::fs::create_dir_all(&directory).unwrap();
        assert!(directory.as_os_str().encode_wide().count() > 300);
        // Ordinary long paths must work without requiring callers to supply
        // Win32 verbatim prefixes or weakening ACL ownership/link checks.
        super::private(&directory, true, true).unwrap();
        let path = directory.join("private-cache.json");
        crate::create_private(&path, b"first").unwrap();
        super::private(&path, false, false).unwrap();
        crate::atomic_private(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        super::private(&path, false, false).unwrap();
    }
    #[test]
    fn native_owner_acl_checks_real_file_type_and_rejects_hardlinks() {
        let root = tempfile::tempdir().unwrap();
        super::private(root.path(), true, true).unwrap();
        let path = root.path().join("private-file");
        std::fs::write(&path, "synthetic fixture").unwrap();
        super::private(&path, false, true).unwrap();
        super::private(&path, false, false).unwrap();
        assert!(super::private(&path, true, false).is_err());
        let linked = root.path().join("hardlink");
        std::fs::hard_link(&path, &linked).unwrap();
        assert!(super::private(&path, false, false).is_err());
        assert!(super::private(&linked, false, false).is_err());
        assert!(crate::atomic_private(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(&linked).unwrap(), b"synthetic fixture");
    }
    #[test]
    fn existing_inherited_or_foreign_acl_is_rejected_without_repair() {
        let root = tempfile::tempdir().unwrap();
        super::private(root.path(), true, true).unwrap();
        let path = root.path().join("private-file");
        std::fs::write(&path, "synthetic fixture").unwrap();
        super::private(&path, false, true).unwrap();
        let user = CurrentUser::load().unwrap();
        let file = open(&path, false, true).unwrap();
        // SAFETY: test changes only its newly created synthetic file and retained handle.
        unsafe {
            code(SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            ))
            .unwrap();
        }
        assert!(super::private(&path, false, false).is_err());
        assert!(validate(file.as_raw_handle(), &user).is_err());
        assert!(crate::atomic_private(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"synthetic fixture");
        set_owner(file.as_raw_handle(), &user, false).unwrap();
        let mut world = null_mut();
        let text = "S-1-1-0\0".encode_utf16().collect::<Vec<_>>();
        // SAFETY: terminated constant SID and a writable output; LocalFree owns the result.
        unsafe {
            ok(ConvertStringSidToSidW(text.as_ptr(), &mut world)).unwrap();
        }
        let world = LocalAllocation(world);
        let mut acl = owner_acl(world.0, false).unwrap();
        // SAFETY: retained test handle permits ACL updates; synthetic world ACE is only a negative fixture.
        unsafe {
            code(SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                acl.as_mut_ptr().cast(),
                null_mut(),
            ))
            .unwrap();
        }
        let denied = super::private(&path, false, false);
        let still_denied = validate(file.as_raw_handle(), &user);
        let replacement_denied = crate::atomic_private(&path, b"replacement").is_err();
        set_owner(file.as_raw_handle(), &user, false).unwrap();
        assert!(denied.is_err() && still_denied.is_err());
        assert!(replacement_denied);
        assert_eq!(std::fs::read(&path).unwrap(), b"synthetic fixture");
        super::private(&path, false, false).unwrap();
    }

    #[test]
    fn protected_null_or_empty_dacl_never_counts_as_private() {
        let root = tempfile::tempdir().unwrap();
        super::private(root.path(), true, true).unwrap();
        let path = root.path().join("synthetic-file");
        std::fs::write(&path, "public synthetic fixture").unwrap();
        super::private(&path, false, true).unwrap();
        let user = CurrentUser::load().unwrap();
        let file = open(&path, false, true).unwrap();
        // SAFETY: only the retained synthetic test object is changed; restore
        // its owner-only DACL before asserting, including the NULL/all-access case.
        unsafe {
            code(SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            ))
            .unwrap();
        }
        let null_rejected = super::private(&path, false, false).is_err();
        let null_replacement_rejected = crate::atomic_private(&path, b"replacement").is_err();
        set_owner(file.as_raw_handle(), &user, false).unwrap();
        assert!(null_rejected && null_replacement_rejected);
        assert_eq!(std::fs::read(&path).unwrap(), b"public synthetic fixture");
        let mut empty = vec![0usize; size_of::<ACL>().div_ceil(size_of::<usize>())];
        // SAFETY: aligned buffer is large enough for an empty ACL; selected handle remains owned.
        unsafe {
            ok(InitializeAcl(
                empty.as_mut_ptr().cast(),
                (empty.len() * size_of::<usize>()) as u32,
                ACL_REVISION,
            ))
            .unwrap();
            code(SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                empty.as_mut_ptr().cast(),
                null_mut(),
            ))
            .unwrap();
        }
        let empty_rejected = super::private(&path, false, false).is_err();
        let empty_replacement_rejected = crate::atomic_private(&path, b"replacement").is_err();
        set_owner(file.as_raw_handle(), &user, false).unwrap();
        assert!(empty_rejected && empty_replacement_rejected);
        assert_eq!(std::fs::read(&path).unwrap(), b"public synthetic fixture");
        super::private(&path, false, false).unwrap();
    }
}
