use crate::Stdio;

use winapi::shared::ntdef::HANDLE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0};

use std::io;
use std::ptr::null_mut;



/// A [WslLaunch]ed Process
///
/// ### See Also
///
/// - [Process]
/// - [Library::launch]
/// - [WslLaunch]
///
/// [Library::launch]:              struct.Library.html#method.launch
/// [WslLaunch]:                    https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wsllaunch
pub struct Process {
    pub(crate) handle: HANDLE,
    pub(crate) stdin:  Stdio,
    pub(crate) stdout: Stdio,
    pub(crate) stderr: Stdio,
}

impl Process {
    /// Waits for the WSL process to exit completely.
    pub fn wait(mut self) -> io::Result<ExitStatus> { self.join_impl() }

    // Also called by `Drop`
    fn join_impl(&mut self) -> io::Result<ExitStatus> {
        assert!(!self.handle.is_null(), "Process::join_impl already called once");

        let wait = unsafe { WaitForSingleObject(self.handle, INFINITE) };
        if wait != WAIT_OBJECT_0 { return Err(std::io::Error::last_os_error()); }

        let handle  = std::mem::replace(&mut self.handle, null_mut());
        let _stdin  = std::mem::replace(&mut self.stdin,  Stdio::null());
        let _stderr = std::mem::replace(&mut self.stderr, Stdio::null());
        let _stdout = std::mem::replace(&mut self.stdout, Stdio::null());

        let succeeded = unsafe { CloseHandle(handle) };
        if succeeded == 0 { return Err(std::io::Error::last_os_error()); }

        Ok(ExitStatus(()))
    }
}

impl std::ops::Drop for Process {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            self.join_impl().expect("winapi error while dropping wslapi::Process");
        }
    }
}



/// <span style="opacity: 33%">(TODO)</span>
/// The exit status of a WSL process.
/// Not yet implemented.
pub struct ExitStatus(());
