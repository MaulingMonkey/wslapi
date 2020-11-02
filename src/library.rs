#![allow(non_snake_case)]

use crate::{Error, Result};
use crate::{Configuration, Process, Stdio};
use crate::WSL_DISTRIBUTION_FLAGS;

use winapi::shared::minwindef::{BOOL, DWORD};
use winapi::shared::ntdef::{HANDLE, PCWSTR, PSTR, ULONG};
use winapi::shared::winerror::{SUCCEEDED, HRESULT, E_INVALIDARG};

use std::convert::TryInto;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null_mut;

// https://github.com/microsoft/WSL/issues/4645 - Wsl* may throw on null pointers, so avoid passing null pointers



/// A loaded `wslapi.dll` or `api-ms-win-wsl-api-l1-1-0.dll` instance
pub struct Library {
    WslIsDistributionRegistered:        unsafe fn (distributionName: PCWSTR) -> BOOL,
    WslRegisterDistribution:            unsafe fn (distributionName: PCWSTR, tarGzFilename: PCWSTR) -> HRESULT,
    WslUnregisterDistribution:          unsafe fn (distributionName: PCWSTR) -> HRESULT,
    WslConfigureDistribution:           unsafe fn (distributionName: PCWSTR, defaultUID: ULONG, wslDistributionFlags: WSL_DISTRIBUTION_FLAGS) -> HRESULT,
    WslGetDistributionConfiguration:    unsafe fn (distributionName: PCWSTR, distributionVersion: *mut ULONG, defaultUID: *mut ULONG, wslDistributionFlags: *mut WSL_DISTRIBUTION_FLAGS, defaultEnvironmentVariables: *mut *mut PSTR, defaultEnvironmentVariableCount: *mut ULONG) -> HRESULT,
    WslLaunchInteractive:               unsafe fn (distributionName: PCWSTR, command: PCWSTR, useCurrentWorkingDirectory: BOOL, exitCode: *mut DWORD) -> HRESULT,
    WslLaunch:                          unsafe fn (distributionName: PCWSTR, command: PCWSTR, useCurrentWorkingDirectory: BOOL, stdIn: HANDLE, stdOut: HANDLE, stdErr: HANDLE, process: *mut HANDLE) -> HRESULT,
}

impl Library {
    /// Attempt to load `wslapi.dll`
    pub fn new() -> io::Result<Self> {
        // fallback on api-ms-win-wsl-api-l1-1-0.dll etc.?
        let lib = minidl::Library::load("wslapi.dll")?;
        unsafe{Ok(Self{
            WslIsDistributionRegistered:        lib.sym("WslIsDistributionRegistered\0")?,
            WslRegisterDistribution:            lib.sym("WslRegisterDistribution\0")?,
            WslUnregisterDistribution:          lib.sym("WslUnregisterDistribution\0")?,
            WslConfigureDistribution:           lib.sym("WslConfigureDistribution\0")?,
            WslGetDistributionConfiguration:    lib.sym("WslGetDistributionConfiguration\0")?,
            WslLaunchInteractive:               lib.sym("WslLaunchInteractive\0")?,
            WslLaunch:                          lib.sym("WslLaunch\0")?,
        })}
    }

    /// Determines if a distribution is registered with the Windows Subsystem for Linux (WSL).
    ///
    /// ### Arguments
    ///
    /// * `distribution_name` - Unique name representing a distribution (for example, "Fabrikam.Distro.10.01").
    ///
    /// ### Returns
    ///
    /// - `true` if the supplied distribution is currently registered
    /// - `false` otherwise.
    ///
    /// ### See Also
    ///
    /// - [WslIsDistributionRegistered] - the underlying API
    ///
    /// [WslIsDistributionRegistered]:  https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wslisdistributionregistered
    pub fn is_distribution_registered(&self, distribution_name: impl AsRef<OsStr>) -> bool {
        let distribution_name = distribution_name.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        !distribution_name[..distribution_name.len()-1].contains(&0) && unsafe { (self.WslIsDistributionRegistered)(distribution_name.as_ptr()) } != 0
    }

    /// Registers a new distribution with the Windows Subsystem for Linux (WSL).
    ///
    /// <span style="color: red">**Consider using `wsl --import <Distro> <InstalLocation> <FileName>` instead:**</span><br>
    /// The directory containing the executable will be registered as the `BasePath` for `rootfs` / `temp` to be placed in.<br>
    /// This odd design choice stems from [WslRegisterDistribution] itself!  Wasn't that a bad choice as far back as Windows XP?<br>
    /// This also limits you to a single registration per executable!
    ///
    /// ### Arguments
    ///
    /// * `distribution_name` - Unique name representing a distribution (for example, "Fabrikam.Distro.10.01").
    /// * `tar_gz_filename` - Full path to a .tar.gz file containing the file system of the distribution to register.
    ///
    /// ### Returns
    ///
    /// - `Err(Error)`  - if `distribution_name` contained `'\0'` characters
    /// - `Err(Error)`  - if `distribution_name` already existed
    /// - `Err(Error)`  - if `tar_gz_filename` contained `'\0'` characters
    /// - `Err(Error)`  - if `tar_gz_filename` wasn't an absolute path?
    /// - `Err(Error)`  - if `tar_gz_filename` wasn't a valid path
    /// - `Err(Error)`  - if the executable's directory already contains a registered distribution
    /// - `Err(Error)`  - if the executable's directory wasn't writable?
    /// - `Err(Error)`  - if [WslRegisterDistribution] otherwise failed
    /// - `Ok(())`      - otherwise
    ///
    /// ### See Also
    ///
    /// - [WslRegisterDistribution] - the underlying API
    ///
    /// [WslRegisterDistribution]:  https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wslregisterdistribution
    pub fn register_distribution(&self, distribution_name: impl AsRef<OsStr>, tar_gz_filename: impl AsRef<Path>) -> Result<()> {
        let wname = distribution_name.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let wpath = tar_gz_filename.as_ref().as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        if wname[..wname.len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("register_distribution({:?}, {:?}) failed: distribution_name contained '\0'", distribution_name.as_ref(), tar_gz_filename.as_ref()) }); }
        if wpath[..wpath.len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("register_distribution({:?}, {:?}) failed: tar_gz_filename contained '\0'",  distribution_name.as_ref(), tar_gz_filename.as_ref()) }); }

        let hr = unsafe { (self.WslRegisterDistribution)(wname.as_ptr(), wpath.as_ptr()) };
        if !SUCCEEDED(hr) { return Err(Error { hresult: hr, message: format!("WslRegisterDistribution({:?}, {:?}) failed with HRESULT 0x{:08x}", distribution_name.as_ref(), tar_gz_filename.as_ref(), hr) }); }
        Ok(())
    }

    /// Unregisters a distribution from the Windows Subsystem for Linux (WSL).
    ///
    /// ### Arguments
    ///
    /// * `distribution_name` - Unique name representing a distribution (for example, "Fabrikam.Distro.10.01").
    ///
    /// ### Returns
    ///
    /// - `Err(Error)`  - if `distribution_name` contained `'\0'` characters
    /// - `Err(Error)`  - if `distribution_name` didn't exist?
    /// - `Err(Error)`  - if [WslUnregisterDistribution] failed
    /// - `Ok(())`      - otherwise
    ///
    /// ### See Also
    ///
    /// - [WslUnregisterDistribution] - the underlying API
    ///
    /// [WslUnregisterDistribution]:        https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wslunregisterdistribution
    pub fn unregister_distribution(&self, distribution_name: impl AsRef<OsStr>) -> Result<()> {
        let wname = distribution_name.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        if wname[..wname.len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("unregister_distribution({:?}) failed: distribution_name contained '\0'", distribution_name.as_ref()) }); }

        let hr = unsafe { (self.WslUnregisterDistribution)(wname.as_ptr()) };
        if !SUCCEEDED(hr) { return Err(Error { hresult: hr, message: format!("WslUnregisterDistribution({:?}) failed with HRESULT 0x{:08x}", distribution_name.as_ref(), hr) }); }
        Ok(())
    }

    /// Modifies the behavior of a distribution registered with the Windows Subsystem for Linux (WSL).
    ///
    /// ### Arguments
    ///
    /// * `distribution_name`       - Unique name representing a distribution (for example, "Fabrikam.Distro.10.01").
    /// * `default_uid`             - The Linux user ID to use when launching new WSL sessions for this distribution.
    /// * `wsl_distribution_flags`  - Flags specifying what behavior to use for this distribution.
    ///
    /// ### Returns
    ///
    /// - `Err(Error)`  - if `distribution_name` contained `'\0'` characters
    /// - `Err(Error)`  - if `distribution_name` didn't exist?
    /// - `Err(Error)`  - if [WslConfigureDistribution] otherwise failed (invalid uid? invalid flags?)
    /// - `Ok(())`      - otherwise
    ///
    /// ### Returns
    ///
    /// [WslConfigureDistribution]:     https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wslconfiguredistribution
    pub fn configure_distribution(&self, distribution_name: impl AsRef<OsStr>, default_uid: ULONG, wsl_distribution_flags: WSL_DISTRIBUTION_FLAGS) -> Result<()> {
        let wname = distribution_name.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        if wname[..wname.len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("configure_distribution({:?}) failed: distribution_name contained '\0'", distribution_name.as_ref()) }); }

        let hr = unsafe { (self.WslConfigureDistribution)(wname.as_ptr(), default_uid, wsl_distribution_flags) };
        if !SUCCEEDED(hr) { return Err(Error { hresult: hr, message: format!("WslConfigureDistribution({:?}, {}, {:?}) failed with HRESULT 0x{:08x}", distribution_name.as_ref(), default_uid, wsl_distribution_flags, hr) }); }
        Ok(())
    }

    /// Retrieves the current configuration of a distribution registered with the Windows Subsystem for Linux (WSL).
    ///
    /// ### Arguments
    ///
    /// * `distribution_name` - Unique name representing a distribution (for example, "Fabrikam.Distro.10.01").
    ///
    /// ### Returns
    ///
    /// - `Err(Error)` - if `distribution_name` contained `'\0'` characters
    /// - `Err(Error)` - if `distribution_name` didn't exist?
    /// - `Err(Error)` - if [WslGetDistributionConfiguration] failed
    /// - `Ok(Configuration { version, default_uid, flags, default_environment_variables })` - otherwise
    ///
    /// ### See Also
    ///
    /// - [WslGetDistributionConfiguration] - the underlying API
    /// - [Configuration] - the returned struct
    ///
    /// [WslGetDistributionConfiguration]:      https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wslgetdistributionconfiguration
    pub fn get_distribution_configuration(&self, distribution_name: impl AsRef<OsStr>) -> Result<Configuration> {
        let wname = distribution_name.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        if wname[..wname.len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("get_distribution_configuration({:?}, ...) failed: distribution_name contained '\0'", distribution_name.as_ref()) }); }

        let mut cfg = Configuration::default();
        let mut nvars = 0;
        let hr = unsafe { (self.WslGetDistributionConfiguration)(wname.as_ptr(), &mut cfg.version, &mut cfg.default_uid, &mut cfg.flags, &mut cfg.default_environment_variables.array, &mut nvars) };
        cfg.default_environment_variables.count = nvars.try_into().unwrap();
        if !SUCCEEDED(hr) { return Err(Error { hresult: hr, message: format!("WslGetDistributionConfiguration({:?}, ...) failed with HRESULT 0x{:08x}", distribution_name.as_ref(), hr) }); }
        Ok(cfg)
    }

    /// Launches an interactive Windows Subsystem for Linux (WSL) process in the context of a particular distribution.
    /// This differs from [Library::launch] in that the end user will be able to interact with the newly-created process.
    ///
    /// ### Arguments
    ///
    /// * `distribution_name` - Unique name representing a distribution (for example, "Fabrikam.Distro.10.01").
    /// * `command` - Command to execute. If no command is supplied, launches the default shell.
    /// * `use_current_working_directory` - Governs whether or not the launched process should inherit
    ///   the calling process's working directory. If `false`, the process is started in the WSL
    ///   default user's home directory ("~").
    ///
    /// ### Returns
    ///
    /// - `Err(Error)`  - if `distribution_name` contained `'\0'` characters
    /// - `Err(Error)`  - if `distribution_name` didn't exist?
    /// - `Err(Error)`  - if `command` contained `'\0'` characters
    /// - `Err(Error)`  - if [WslLaunchInteractive] otherwise failed
    /// - `Ok(DWORD)`   - the exit code of the process after it exits.
    ///
    /// ### See Also
    ///
    /// - [Library::launch] - non-interactive, programatic interaction
    /// - [WslLaunchInteractive] - the underlying API
    ///
    /// [Library::launch]:      crate::Library::launch
    /// [WslLaunchInteractive]: https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wsllaunchinteractive
    pub fn launch_interactive(&self, distribution_name: impl AsRef<OsStr>, command: impl AsRef<OsStr>, use_current_working_directory: bool) -> Result<DWORD> {
        let wname = distribution_name.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let wcmd  = command.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        if wname[..wname.len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("launch_interactive({:?}, {:?}, {}) failed: distribution_name contained '\0'",  distribution_name.as_ref(), command.as_ref(), use_current_working_directory) }); }
        if wcmd [..wcmd .len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("launch_interactive({:?}, {:?}, {}) failed: command contained '\0'",            distribution_name.as_ref(), command.as_ref(), use_current_working_directory) }); }

        let mut exit_code = 0;
        let hr = unsafe { (self.WslLaunchInteractive)(wname.as_ptr(), if command.as_ref().is_empty() { null_mut() } else { wcmd.as_ptr() }, use_current_working_directory as BOOL, &mut exit_code) };
        if !SUCCEEDED(hr) { return Err(Error { hresult: hr, message: format!("WslLaunchInteractive({:?}, {:?}, {}) failed with HRESULT 0x{:08x}", distribution_name.as_ref(), command.as_ref(), use_current_working_directory, hr) }); }
        Ok(exit_code)
    }

    /// Launches a Windows Subsystem for Linux (WSL) process in the context of a particular distribution.
    ///
    /// ### Arguments
    ///
    /// * `distribution_name`   - Unique name representing a distribution (for example, "Fabrikam.Distro.10.01").
    /// * `command`             - Command to execute. If no command is supplied, launches the default shell.
    /// * `use_current_working_directory` - Governs whether or not the launched process should inherit
    ///   the calling process's working directory. If `false`, the process is started in the WSL
    ///   default user's home directory ("~").
    /// * `stdin`               - Handle to use for **STDIN**.
    /// * `stdout`              - Handle to use for **STDOUT**.
    /// * `stderr`              - Handle to use for **STDERR**.
    ///
    /// ### Returns
    ///
    /// - `Err(Error)`  - if `distribution_name` contained `'\0'` characters
    /// - `Err(Error)`  - if `distribution_name` didn't exist?
    /// - `Err(Error)`  - if `command` contained `'\0'` characters
    /// - `Err(Error)`  - if `stdin`, `stdout`, or `stderr` failed to convert to [Stdio]
    /// - `Err(Error)`  - if `stdin`, `stdout`, or `stderr` was an invalid handle for [WslLaunch]
    /// - `Err(Error)`  - if [WslLaunch] otherwise failed
    /// - `Ok(Process)` - if the WSL process that launched successfully
    ///
    /// ### See Also
    ///
    /// - [Process]
    /// - [Library::launch_interactive] - interactive, inherits the same console handles
    /// - [WslLaunch] - the underlying API
    ///
    /// [Library::launch_interactive]:  #method.launch_interactive
    /// [WslLaunch]:                    https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wsllaunch
    pub fn launch<I, O, E>(
        &self,
        distribution_name:              impl AsRef<OsStr>,
        command:                        impl AsRef<OsStr>,
        use_current_working_directory:  bool,
        stdin:                          I,
        stdout:                         O,
        stderr:                         E,
    ) -> Result<Process> where
        I : TryInto<Stdio>, I::Error : Display,
        O : TryInto<Stdio>, O::Error : Display,
        E : TryInto<Stdio>, E::Error : Display,
    {
        // https://github.com/microsoft/WSL-DistroLauncher/blob/540a593313f8abbc8ce3afe8ca98434e8a771798/DistroLauncher/DistributionInfo.cpp#L48

        let wname = distribution_name.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let wcmd  = command.as_ref().encode_wide().chain(Some(0)).collect::<Vec<_>>();
        if wname[..wname.len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("launch({:?}, {:?}, {}, ...) failed: distribution_name contained '\0'",  distribution_name.as_ref(), command.as_ref(), use_current_working_directory) }); }
        if wcmd [..wcmd .len()-1].contains(&0) { return Err(Error { hresult: E_INVALIDARG, message: format!("launch({:?}, {:?}, {}, ...) failed: command contained '\0'",            distribution_name.as_ref(), command.as_ref(), use_current_working_directory) }); }
        let stdin  = stdin .try_into().map_err(|err| Error { hresult: E_INVALIDARG, message: format!("launch({:?}, {:?}, {}, ...) failed: failed to convert stdin: {}",  distribution_name.as_ref(), command.as_ref(), use_current_working_directory, err) })?;
        let stdout = stdout.try_into().map_err(|err| Error { hresult: E_INVALIDARG, message: format!("launch({:?}, {:?}, {}, ...) failed: failed to convert stdout: {}", distribution_name.as_ref(), command.as_ref(), use_current_working_directory, err) })?;
        let stderr = stderr.try_into().map_err(|err| Error { hresult: E_INVALIDARG, message: format!("launch({:?}, {:?}, {}, ...) failed: failed to convert stderr: {}", distribution_name.as_ref(), command.as_ref(), use_current_working_directory, err) })?;

        let mut handle = null_mut();
        let hr = unsafe { (self.WslLaunch)(wname.as_ptr(), wcmd.as_ptr(), use_current_working_directory as BOOL, stdin.as_winapi_handle(), stdout.as_winapi_handle(), stderr.as_winapi_handle(), &mut handle) };
        if !SUCCEEDED(hr) { return Err(Error { hresult: hr, message: format!("WslLaunch({:?}, {:?}, {}, ...) failed with HRESULT 0x{:08x}", distribution_name.as_ref(), command.as_ref(), use_current_working_directory, hr) }); }
        Ok(Process { handle, stdin, stdout, stderr })
    }
}
