#![allow(non_camel_case_types)] // WSL_DISTRIBUTION_FLAGS

use std::fmt::{self, Debug, Formatter};
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};



/// Flags specifying WSL behavior
///
/// \[[docs.microsoft.com](https://docs.microsoft.com/en-us/windows/win32/api/wslapi/ne-wslapi-wsl_distribution_flags)\]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)] pub struct WSL_DISTRIBUTION_FLAGS(u32);

impl WSL_DISTRIBUTION_FLAGS {
    /// No flags are being supplied.
    pub const NONE                      : Self = Self(0);

    /// Allow the distribution to interoperate with Windows processes (for example,
    /// the user can invoke `cmd.exe` or `notepad.exe` from within a WSL session).
    pub const ENABLE_INTEROP            : Self = Self(0x1);

    /// Add the Windows `%PATH%` environment variable values to WSL sessions.
    pub const APPEND_NT_PATH            : Self = Self(0x2);

    /// Automatically mount Windows drives inside of WSL sessions (for example,
    /// `C:` will be available under `/mnt/c`).
    pub const ENABLE_DRIVE_MOUNTING     : Self = Self(0x4);

    /// All valid flags
    pub const VALID                     : Self = Self(0x7);

    /// Default flags (all valid flags)
    pub const DEFAULT                   : Self = Self(0x7);
}

impl BitAnd for WSL_DISTRIBUTION_FLAGS {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self { Self(self.0 & rhs.0) }
}

impl BitAndAssign for WSL_DISTRIBUTION_FLAGS {
    fn bitand_assign(&mut self, rhs: Self) { self.0 &= rhs.0; }
}

impl BitOr for WSL_DISTRIBUTION_FLAGS {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}

impl BitOrAssign for WSL_DISTRIBUTION_FLAGS {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

impl Default for WSL_DISTRIBUTION_FLAGS {
    fn default() -> Self { Self::DEFAULT }
}

impl From<u32> for WSL_DISTRIBUTION_FLAGS {
    fn from(value: u32) -> Self { Self(value) }
}

impl From<WSL_DISTRIBUTION_FLAGS> for u32 {
    fn from(value: WSL_DISTRIBUTION_FLAGS) -> Self { value.0 }
}

impl Debug for WSL_DISTRIBUTION_FLAGS {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        if self.0 == 0 {
            write!(fmt, "WSL_DISTRIBUTION_FLAGS::NONE")
        } else {
            write!(fmt, "WSL_DISTRIBUTION_FLAGS::")?;
            let n = if self.0 == 0x7 { 1 } else { (self.0 & 0x7).count_ones() + (self.0 & !0x7).min(1) };
            if n > 1 { write!(fmt, "(")?; }
            let mut prev = false;
            if self.0 & 0x7 == 7 {
                prev = true;
                write!(fmt, "DEFAULT")?;
            } else {
                if self.0 & 0x1 != 0 { if prev { write!(fmt, "|")?; } prev = true; write!(fmt, "ENABLE_INTEROP")?; }
                if self.0 & 0x2 != 0 { if prev { write!(fmt, "|")?; } prev = true; write!(fmt, "APPEND_NT_PATH")?; }
                if self.0 & 0x4 != 0 { if prev { write!(fmt, "|")?; } prev = true; write!(fmt, "ENABLE_DRIVE_MOUNTING")?; }
            }
            let invalid = self.0 & !Self::VALID.0;
            if invalid != 0 { if prev { write!(fmt, "|")?; } write!(fmt, "0x{:X}", invalid)?; }
            if n > 1 { write!(fmt, ")")?; }
            Ok(())
        }
    }
}

#[test] fn fmt_debug() {
    assert_eq!("WSL_DISTRIBUTION_FLAGS::NONE",                              format!("{:?}", WSL_DISTRIBUTION_FLAGS::NONE));
    assert_eq!("WSL_DISTRIBUTION_FLAGS::(ENABLE_INTEROP|APPEND_NT_PATH)",   format!("{:?}", WSL_DISTRIBUTION_FLAGS::ENABLE_INTEROP | WSL_DISTRIBUTION_FLAGS::APPEND_NT_PATH));
    assert_eq!("WSL_DISTRIBUTION_FLAGS::DEFAULT",                           format!("{:?}", WSL_DISTRIBUTION_FLAGS::DEFAULT));
    assert_eq!("WSL_DISTRIBUTION_FLAGS::(DEFAULT|0xFFFFFFF8)",              format!("{:?}", WSL_DISTRIBUTION_FLAGS(!0)));
}
