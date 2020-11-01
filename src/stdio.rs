use winapi::shared::ntdef::HANDLE;
use winapi::um::handleapi::{CloseHandle, DuplicateHandle};
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::winbase::FILE_FLAG_DELETE_ON_CLOSE;
use winapi::um::winnt::{DUPLICATE_CLOSE_SOURCE, DUPLICATE_SAME_ACCESS, FILE_ATTRIBUTE_TEMPORARY};

use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::os::windows::prelude::*;
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::ops::Drop;
use std::sync::atomic::{AtomicUsize, Ordering::AcqRel};
use std::ptr::null_mut;


static COUNTER : AtomicUsize = AtomicUsize::new(0);

/// A [WslLaunch] stdin, stdout, or stderr parameter
///
/// [WslLaunch]:                    https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wsllaunch
pub struct Stdio {
    owner:  Option<Box<dyn AsRawHandle>>,
}

impl Stdio {
    /// This stream will be ignored. This is the equivalent of attaching the stream to `/dev/null`
    pub fn null() -> Self { Self { owner: None } }

    /// Stream data from a temporary file containing the contents of `bytes`
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> io::Result<Self> {
        let path = std::env::temp_dir().join(format!("wslapi-{}-{}.tmp", std::process::id(), COUNTER.fetch_add(1, AcqRel)));

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .read(true)
            .attributes(FILE_ATTRIBUTE_TEMPORARY)       // prefer in-memory cache
            .custom_flags(FILE_FLAG_DELETE_ON_CLOSE)    // cleanup after use
            .open(&path)?;

        file.write_all(bytes.as_ref())?;
        file.seek(SeekFrom::Start(0))?;

        Self::from_file(file)
    }

    /// Stream data from or into a file handle
    pub fn from_file(file: File) -> io::Result<Self> {
        let proc = unsafe { GetCurrentProcess() };
        let mut handle = null_mut();
        let success = unsafe { DuplicateHandle(proc, file.into_raw_handle().cast(), proc, &mut handle, 0, 1, DUPLICATE_CLOSE_SOURCE | DUPLICATE_SAME_ACCESS) };
        if success == 0 { return Err(io::Error::last_os_error()) }
        Ok(unsafe { Self::from_handle(handle) })
    }

    /// Take ownership of a raw handle
    ///
    /// # <span style="color: red">Safety</span>
    ///
    /// `handle` is assumed to be "valid".  That is:
    ///
    /// * [CloseHandle]\(handle\) must be sound when [Stdio] is [Drop]ped
    /// * [WslLaunch]\(..., handle, ...\) must be legal for stdIn, stdOut, or stdErr
    /// * The handle must be [inheritable] *without* [CreateProcess]'s `bInheritHandles`=`TRUE`
    /// * `NULL` is also legal
    ///
    /// [CloseHandle]:          https://docs.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle
    /// [CreateProcess]:        https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
    /// [Drop]:                 https://doc.rust-lang.org/std/ops/trait.Drop.html
    /// [inheritable]:          https://docs.microsoft.com/en-us/windows/win32/sysinfo/handle-inheritance
    /// [WslLaunch]:            https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wsllaunch
    pub unsafe fn from_handle(handle: HANDLE) -> Self {
        Self { owner: Some(Box::new(OwnHandle(handle))) }
    }

    /// Take ownership of something with a raw handle
    ///
    /// # <span style="color: red">Safety</span>
    ///
    /// `owner.as_raw_handle()` is assumed to be "valid".  That is:
    ///
    /// * [WslLaunch]\(..., owner.as_raw_handle(), ...\) must be legal for stdIn, stdOut, or stdErr
    /// * The handle must be [inheritable] *without* [CreateProcess]'s `bInheritHandles`=`TRUE`
    ///
    /// [CloseHandle]:          https://docs.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle
    /// [CreateProcess]:        https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
    /// [inheritable]:          https://docs.microsoft.com/en-us/windows/win32/sysinfo/handle-inheritance
    /// [WslLaunch]:            https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wsllaunch
    pub unsafe fn from_as_raw_handle(owner: impl AsRawHandle + 'static) -> Self {
        Self { owner: Some(Box::new(owner)) }
    }

    /// Get a standard std::os::windows::{io::[RawHandle] / raw::[HANDLE]}
    ///
    /// [RawHandle]:            https://doc.rust-lang.org/std/os/windows/io/type.RawHandle.html
    /// [HANDLE]:               https://doc.rust-lang.org/std/os/windows/raw/type.HANDLE.html
    pub fn as_raw_handle(&self) -> RawHandle {
        self.owner.as_ref().map_or(null_mut(), |owner| owner.as_raw_handle())
    }

    /// Get a winapi::shared::ntdef::[HANDLE]
    ///
    /// [HANDLE]:               https://docs.rs/winapi/0.3/winapi/shared/ntdef/type.HANDLE.html
    pub fn as_winapi_handle(&self) -> winapi::shared::ntdef::HANDLE {
        self.owner.as_ref().map_or(null_mut(), |owner| owner.as_raw_handle()).cast()
    }
}

impl AsRawHandle for Stdio {
    fn as_raw_handle(&self) -> RawHandle { self.owner.as_ref().map_or(null_mut(), |owner| owner.as_raw_handle()) }
}

// XXX:
// https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html
// https://docs.microsoft.com/en-us/windows/win32/api/fileapi/ns-fileapi-createfile2_extended_parameters
// https://docs.microsoft.com/en-us/previous-versions/windows/desktop/legacy/aa379560(v=vs.85)
//
// TL;DR:  Need inheritable HANDLEs, not just in-process HANDLEs

impl From<()>           for Stdio { fn from(_value: ())     -> Self { Self::null() } }

// "Console handles can be duplicated for use only in the same process."
//impl From<Stderr>       for Stdio { fn from(value: Stderr)  -> Self { Self { owner: Some(Box::new(value)) } } }
//impl From<Stdin >       for Stdio { fn from(value: Stdin )  -> Self { Self { owner: Some(Box::new(value)) } } }
//impl From<Stdout>       for Stdio { fn from(value: Stdout)  -> Self { Self { owner: Some(Box::new(value)) } } }

impl TryFrom<File>      for Stdio { fn try_from(value: File) -> io::Result<Self> { Self::from_file( value) } type Error = io::Error; }

impl TryFrom<Vec<u8>>   for Stdio { fn try_from(value: Vec<u8>) -> io::Result<Self> { Self::from_bytes(&value) } type Error = io::Error; }
impl TryFrom<&[u8]>     for Stdio { fn try_from(value: &[u8])   -> io::Result<Self> { Self::from_bytes(value)  } type Error = io::Error; }
impl TryFrom<String>    for Stdio { fn try_from(value: String)  -> io::Result<Self> { Self::from_bytes(&value) } type Error = io::Error; }
impl TryFrom<&str>      for Stdio { fn try_from(value: &str)    -> io::Result<Self> { Self::from_bytes(value)  } type Error = io::Error; }



struct OwnHandle(HANDLE);

impl AsRawHandle for OwnHandle {
    fn as_raw_handle(&self) -> RawHandle { self.0.cast() }
}

impl Drop for OwnHandle {
    fn drop(&mut self) {
        if self.0.is_null() { return }
        let succeeded = unsafe { CloseHandle(self.0) };
        assert_ne!(0, succeeded, "CloseHandle(0x{:08x}) failed: {:?}", self.0 as usize, std::io::Error::last_os_error());
    }
}
