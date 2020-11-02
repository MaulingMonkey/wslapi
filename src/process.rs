use crate::Stdio;

use winapi::shared::minwindef::DWORD;
use winapi::shared::ntdef::HANDLE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::GetExitCodeProcess;
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

        let mut exit_code = 0;
        let succeeded = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
        let exit_code = if succeeded != 0 { Some(exit_code) } else { None };

        let succeeded = unsafe { CloseHandle(handle) };
        if succeeded == 0 { return Err(std::io::Error::last_os_error()); }

        Ok(ExitStatus { exit_code })
    }
}

impl std::ops::Drop for Process {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            self.join_impl().expect("winapi error while dropping wslapi::Process");
        }
    }
}



/// The exit status of a WSL process.
pub struct ExitStatus {
    exit_code:  Option<DWORD>,
}

impl ExitStatus {
    /// Was termination successful?
    pub fn success(&self) -> bool { self.exit_code == Some(0) }

    /// Returns the exit code of the process, if any.
    pub fn code(&self) -> Option<DWORD> { self.exit_code }
    // While POSIX truncates the result to 1 byte / 8 bits / 0xFF, it's possible
    // that the WSL process itself could crash/fail/kernel panic/??? with other
    // exit code results.  As such, I retain the API.  Unlike std::process::ExitCode,
    // the mapped code in question is *unsigned*.
}
