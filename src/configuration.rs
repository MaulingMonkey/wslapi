use crate::WSL_DISTRIBUTION_FLAGS;

use winapi::shared::ntdef::{PSTR, ULONG};
use winapi::um::combaseapi::CoTaskMemFree;

use std::ops::Drop;
use std::ptr::null_mut;

// winapi uses `CHAR` = `c_char` = `i8`, but `u8` is way more rusty considering
// the env vars in question might actually be UTF8, so I expose `&[u8]` instead
// of `&[CHAR]` / `&[i8]`.  Of course, they could be ASCII or another locale
// entirely as well, so I don't try to `&str`ify the env strings in question.



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



/// The environment variables of [WslGetDistributionConfiguration].
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

    /// Get the key/value pair at `index`
    pub fn get(&self, index: usize) -> Option<(&[u8], &[u8])> {
        if index >= self.count {
            None
        } else {
            let st = unsafe { *self.array.add(index) };

            for i in 0.. {
                match unsafe { *st.add(i) } as _ {
                    b'\0' => return Some((unsafe { std::slice::from_raw_parts(st.cast(), i) }, &[])),
                    b'=' => for k in i.. {
                        if unsafe { *st.add(k) } != 0 { continue }

                        let all = unsafe { std::slice::from_raw_parts(st.cast(), k) };
                        let (k, v) = all.split_at(i);
                        return Some((k, &v[1..]));
                    },
                    _ => {},
                }
            }

            None // unreachable
        }
    }

    /// Iterate over the key/value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &[u8])> {
        EnvironmentVariablesIter { vars: self, index: 0 }
    }
}

impl Default for EnvironmentVariables {
    fn default() -> Self { Self::new() }
}

impl Drop for EnvironmentVariables {
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

impl<'e> IntoIterator for &'e EnvironmentVariables {
    type Item = (&'e [u8], &'e [u8]);
    type IntoIter = EnvironmentVariablesIter<'e>;
    fn into_iter(self) -> Self::IntoIter {
        EnvironmentVariablesIter { vars: self, index: 0 }
    }
}



/// Iterator over &[EnvironmentVariables]
pub struct EnvironmentVariablesIter<'e> {
    vars:   &'e EnvironmentVariables,
    index:  usize,
}

impl<'e> Iterator for EnvironmentVariablesIter<'e> {
    type Item = (&'e [u8], &'e [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        let kv = self.vars.get(self.index)?;
        self.index += 1;
        Some(kv)
    }
}
