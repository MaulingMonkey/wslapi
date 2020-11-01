use crate::WSL_DISTRIBUTION_FLAGS;

use winapi::shared::ntdef::ULONG;
use winapi::shared::ntdef::PSTR;
use winapi::um::combaseapi::CoTaskMemFree;

use std::ptr::null_mut;



#[derive(Default)]
/// The structified result of [WslGetDistributionConfiguration]
/// 
/// [WslGetDistributionConfiguration]:      https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wslgetdistributionconfiguration
pub struct Configuration {
    /// The version of WSL for which this distribution is configured.
    pub version:                        ULONG,

    /// The default user ID used when launching new WSL sessions for this distribution.
    pub default_uid:                    ULONG,

    /// The flags governing the behavior of this distribution.
    pub flags:                          WSL_DISTRIBUTION_FLAGS,

    /// The default environment variable strings used when launching new WSL sessions for this distribution.
    pub default_environment_variables:  EnvironmentVariables,
}



/// <span style="opacity: 33%">(TODO)</span>
/// The environment variables of [WslGetDistributionConfiguration].
/// Not yet implemented.
/// 
/// [WslGetDistributionConfiguration]:      https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wslgetdistributionconfiguration
pub struct EnvironmentVariables {
    pub(crate) array:  *mut PSTR,
    pub(crate) count:  usize,
}

impl EnvironmentVariables {
    /// Create an empty/null array of environment variables
    pub fn new() -> Self { Self { array: null_mut(), count: 0 } }

    /// Get the number of environment variables
    pub fn len(&self) -> usize { self.count }
}

impl Default for EnvironmentVariables {
    fn default() -> Self { Self::new() }
}

impl std::ops::Drop for EnvironmentVariables {
    fn drop(&mut self) {
        // "The caller is responsible for freeing each string in pDefaultEnvironmentVariablesArray (and the array itself) via CoTaskMemFree."
        // https://docs.microsoft.com/en-us/windows/win32/api/wslapi/nf-wslapi-wslgetdistributionconfiguration

        if !self.array.is_null() {
            for i in 0..self.count {
                unsafe { CoTaskMemFree((*self.array.add(i)).cast()) };
            }
            unsafe { CoTaskMemFree(self.array.cast()) }
        }

        self.array = null_mut();
        self.count = 0;
    }
}
