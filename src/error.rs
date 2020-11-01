use winapi::shared::winerror::*;

use std::fmt::{self, Debug, Display, Formatter};
use std::io;



/// A crate Result.
pub type Result<T> = std::result::Result<T, Error>;

/// A crate error.  Convertable to [std::io::Error], Box<dyn [std::error::Error]>
pub struct Error {
    pub(crate) hresult: HRESULT,
    pub(crate) message: String,
}

impl std::error::Error for Error {}

impl Debug for Error {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("wslapi::Error")
            .field("hresult", &HR(self.hresult))
            .field("message", &self.message)
            .finish()
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.message, fmt)
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(hr2ek(err.hresult), err.message)
    }
}



struct HR(HRESULT);
impl Debug for HR {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "{:08x}", self.0)
    }
}

fn hr2ek(hr: HRESULT) -> io::ErrorKind {
    use winapi::shared::winerror::*;

    // https://en.wikipedia.org/wiki/HRESULT
    let hru = hr as u32;
    let _failure = (hru >> 31) & 1 != 0;
    // R, C, N, X
    let facility = (hr >> 16) & 0x7FF;
    let code     = hru & 0xFFFF;

    #[deny(unreachable_patterns)]
    match hr {
        E_INVALIDARG  => io::ErrorKind::InvalidInput,
        _other => match (facility, code) {
            (FACILITY_WIN32, ERROR_ALREADY_EXISTS)      => io::ErrorKind::AlreadyExists,
            (FACILITY_WIN32, ERROR_FILE_NOT_FOUND)      => io::ErrorKind::NotFound,
            (FACILITY_WIN32, ERROR_PATH_NOT_FOUND)      => io::ErrorKind::NotFound,
            (FACILITY_WIN32, ERROR_MOD_NOT_FOUND)       => io::ErrorKind::NotFound,
            (FACILITY_WIN32, ERROR_PROC_NOT_FOUND)      => io::ErrorKind::NotFound,
            (FACILITY_WIN32, ERROR_INVALID_HANDLE)      => io::ErrorKind::InvalidInput,
            (FACILITY_WIN32, ERROR_INVALID_DATA)        => io::ErrorKind::InvalidData,
            (FACILITY_WIN32, ERROR_INVALID_DRIVE)       => io::ErrorKind::InvalidInput,
            (FACILITY_WIN32, ERROR_INVALID_PARAMETER)   => io::ErrorKind::InvalidInput,
            (FACILITY_WIN32, ERROR_INVALID_NAME)        => io::ErrorKind::InvalidInput,
            (FACILITY_WIN32, ERROR_INVALID_LEVEL)       => io::ErrorKind::InvalidInput,
            (FACILITY_WIN32, ERROR_NO_MORE_FILES)       => io::ErrorKind::UnexpectedEof,
            (FACILITY_WIN32, ERROR_WRITE_PROTECT)       => io::ErrorKind::PermissionDenied,
            (FACILITY_WIN32, ERROR_SHARING_VIOLATION)   => io::ErrorKind::PermissionDenied,
            (FACILITY_WIN32, ERROR_LOCK_VIOLATION)      => io::ErrorKind::PermissionDenied,
            (FACILITY_WIN32, ERROR_HANDLE_EOF)          => io::ErrorKind::UnexpectedEof,
            (FACILITY_WIN32, ERROR_FILE_EXISTS)         => io::ErrorKind::AlreadyExists,
            (FACILITY_WIN32, ERROR_BROKEN_PIPE)         => io::ErrorKind::BrokenPipe,
            (FACILITY_WIN32, ERROR_PIPE_NOT_CONNECTED)  => io::ErrorKind::BrokenPipe,
            (FACILITY_WIN32, WAIT_TIMEOUT)              => io::ErrorKind::TimedOut,
            (FACILITY_WIN32, ERROR_SEM_TIMEOUT)         => io::ErrorKind::TimedOut,
            _other                                      => io::ErrorKind::Other,
        },
    }
}
