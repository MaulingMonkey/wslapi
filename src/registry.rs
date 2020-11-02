//! wslapi.h "adjacent" registry keys to fill in where wslapi.h is missing functionality

#![deny(unreachable_patterns)]

use winapi::shared::minwindef::{DWORD, HKEY};
use winapi::shared::winerror::*;
use winapi::um::winbase::{FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM};
use winapi::um::winnt::KEY_ENUMERATE_SUB_KEYS;
use winapi::um::winreg::*;

use std::convert::{TryFrom, TryInto};
use std::ffi::OsString;
use std::ptr::null_mut;
use std::os::windows::prelude::*;



/// Get the `DistributionName`s of all registered WSL distributions from
/// `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Lxss\{...}\DistributionName`
///
/// # Example
///
/// ```rust
/// let library = wslapi::Library::new();
///
/// for distro in wslapi::registry::distribution_names() {
///     let library = library.as_ref().unwrap_or_else(|err| panic!(
///         "WSL not available despite having WSL distributions: {}", err
///     ));
///     assert!(
///         library.is_distribution_registered(&distro),
///         "*not* registered: {}",
///         distro.to_string_lossy()
///     );
/// }
/// ```
pub fn distribution_names() -> impl Iterator<Item = OsString> { DistributionNames::new() }



struct DistributionNames {
    lxss:   HKEY,
    index:  DWORD,
}

impl std::ops::Drop for DistributionNames {
    fn drop(&mut self) { self.close() }
}

impl DistributionNames {
    fn new() -> Self {
        let mut result = null_mut();
        let path = wchar::wch_c!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Lxss");
        let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, KEY_ENUMERATE_SUB_KEYS, &mut result) };
        match status as _ {
            ERROR_SUCCESS           => Self { lxss: result, index: 0 },
            ERROR_FILE_NOT_FOUND    => Self { lxss: null_mut(), index: 0 }, // No WSL installed?
            err                     => panic!("RegOpenKeyExW(HKEY_CURRENT_USER, r\"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Lxss\", ...) failed with error {}", format_message(err)),
        }
    }

    fn close(&mut self) {
        if !self.lxss.is_null() {
            let status = unsafe { RegCloseKey(self.lxss) };
            assert_eq!(ERROR_SUCCESS, status as _, "RegCloseKey(self.lxss) failed with error 0x{:04x})", status);
            self.lxss = null_mut();
        }
    }
}

impl Iterator for DistributionNames {
    type Item = OsString;
    fn next(&mut self) -> Option<OsString> {
        if self.lxss.is_null() { return None }

        let mut key_name = [0u16; 256]; // https://docs.microsoft.com/en-us/windows/win32/sysinfo/registry-element-size-limits
        let mut key_len = key_name.len().try_into().unwrap();
        let status = unsafe { RegEnumKeyExW(self.lxss, self.index, key_name.as_mut_ptr(), &mut key_len, null_mut(), null_mut(), null_mut(), null_mut()) };
        match status as _ {
            ERROR_SUCCESS => {
                self.index += 1;
                let mut value = [0u16; 64 * 1024]; // 64 KiB should be enough for a distro name, probably, right?
                let mut value_len = value.len().try_into().unwrap();
                let status = unsafe { RegGetValueW(self.lxss, key_name.as_ptr(), wchar::wch_c!("DistributionName").as_ptr(), RRF_RT_REG_SZ, null_mut(), value.as_mut_ptr().cast(), &mut value_len) };
                match status as _ {
                    ERROR_SUCCESS   => Some(OsString::from_wide(&value[..(usize::try_from(value_len).unwrap()/2-1)])),
                    err             => panic!("RegGetValueW(self.lxss, \"{{...}}\", \"DistributionName\", ...) failed with error {}", format_message(err)),
                }
            },
            ERROR_NO_MORE_ITEMS => {
                self.close();
                None
            },
            err => panic!("RegEnumKeyExW(self.lxss, ...) failed with error {}", format_message(err)),
        }
    }
}

fn format_message(err: DWORD) -> String {
    // https://docs.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-formatmessage
    let mut buffer = [0u16; 32 * 1024]; // 64 KiB.  "This buffer cannot be larger than 64K bytes."
    let tchars = unsafe { FormatMessageW(FORMAT_MESSAGE_FROM_SYSTEM, null_mut(), err as _, 0, buffer.as_mut_ptr(), buffer.len().try_into().unwrap(), null_mut()) };
    if tchars == 0 {
        format!("0x{:04x}", err)
    } else {
        let tchars = usize::try_from(tchars).unwrap();
        let msg = OsString::from_wide(&buffer[..tchars]);
        format!("0x{:04x}: {}", err, msg.to_string_lossy().trim_end())
    }
}
