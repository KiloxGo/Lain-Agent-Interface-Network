//! Ownership of a subprocess tree, not a general shell execution facility.
use anyhow::{Context, Result};
use tokio::process::{Child, Command};

pub(super) fn configure(command: &mut Command) {
    command.kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
}

#[cfg(unix)]
pub(super) struct ProcessTree(i32);

#[cfg(unix)]
impl ProcessTree {
    pub fn attach(child: &Child) -> Result<Self> {
        Ok(Self(child.id().context("missing child PID")?.try_into()?))
    }
}

#[cfg(unix)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        // SAFETY: the child was placed in its own process group before exec.
        unsafe {
            libc::kill(-self.0, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
pub(super) struct ProcessTree(std::os::windows::io::OwnedHandle);

#[cfg(windows)]
impl ProcessTree {
    pub fn attach(child: &Child) -> Result<Self> {
        use std::{
            mem::{size_of, zeroed},
            os::windows::io::{AsRawHandle, FromRawHandle},
        };
        use windows_sys::Win32::System::JobObjects::*;
        // SAFETY: each Win32 call receives a valid owned handle and initialized buffer.
        // OwnedHandle closes the job on every return path; KILL_ON_JOB_CLOSE reaps descendants.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error()).context("create worker job");
            }
            let job = std::os::windows::io::OwnedHandle::from_raw_handle(job);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
            {
                return Err(std::io::Error::last_os_error()).context("configure worker job");
            }
            if AssignProcessToJobObject(
                job.as_raw_handle(),
                child.raw_handle().context("missing worker handle")?,
            ) == 0
            {
                return Err(std::io::Error::last_os_error()).context("assign worker job");
            }
            Ok(Self(job))
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        use std::os::windows::io::AsRawHandle;
        // SAFETY: this is our still-open job, containing only the owned worker tree.
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0.as_raw_handle(), 1);
        }
    }
}
