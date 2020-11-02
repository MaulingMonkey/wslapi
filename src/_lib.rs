#![cfg(windows)]
#![deny(missing_docs)]

//! APIs for managing the Windows Subsystem for Linux ([wslapi.h])
//!
//! ```rust
//! use wslapi::*;
//!
//! let wsl = Library::new().unwrap();
//!
//! let nonexistant = "Nonexistant";
//! assert!(!wsl.is_distribution_registered(nonexistant));
//! assert!(wsl.get_distribution_configuration(nonexistant).is_err());
//!
//! let mut found = 0;
//! for distro in registry::distribution_names() {
//!     if !wsl.is_distribution_registered(&distro) { continue }
//!     found += 1;
//!
//!     let c = wsl.get_distribution_configuration(&distro).unwrap();
//!     assert!(c.default_uid == 0 || (1000 ..= 2000).contains(&c.default_uid));
//!     // 0 == root, 1000+ == regular user
//!     assert!(c.flags & WSL_DISTRIBUTION_FLAGS::DEFAULT == WSL_DISTRIBUTION_FLAGS::DEFAULT);
//!     // `c.flags` contains extra, undocumented flags like 0x8
//!     assert!((1..=2).contains(&c.version)); // WSL version
//!
//!     wsl.launch_interactive(&distro, "echo testing 123", true).unwrap();
//!
//!     let stdin  = "echo testing 456\necho PATH: ${PATH}\n";
//!     let stdout = std::fs::File::create("target/basic.txt").unwrap();
//!     let stderr = (); // shorthand for Stdio::null()
//!     wsl.launch(&distro, "sh", true, stdin, stdout, stderr).unwrap().wait().unwrap();
//!
//!     for exit in [0, 1, 2, 3, 0xFF, 0x100, 0x101, 0x1FF].iter().copied() {
//!         let script = format!("exit {}", exit);
//!         let wsl = wsl.launch(&distro, "sh", true, script, (), ()).unwrap();
//!         let code = wsl.wait().unwrap().code().unwrap();
//!         if code == !0 {
//!             // May happen if WSL doesn't know the exit code?  On my machine, this
//!             // only happens for Ubuntu 20.04, but on appveyor this happens on Ubuntu-18.04
//!             // or Ubuntu-16.04 as well.
//!         } else if code == 1 && distro == "docker-desktop-data" {
//!             // Not launchable?
//!         } else if exit & 0xFF == 0 {
//!             // 0x100 may be truncated to 0, or coerced to something else
//!             assert!(
//!                 code == exit || code == 0,
//!                 "distro {}, exit {} != code {}",
//!                 distro.to_string_lossy(), exit, code
//!             );
//!         } else {
//!             // 0x101 may be truncated to 1 per POSIX
//!             assert!(
//!                 code == exit || code == exit & 0xFF,
//!                 "distro {}, exit {} != code {}",
//!                 distro.to_string_lossy(), exit, code
//!             );
//!         }
//!     }
//! }
//! assert_ne!(0, found, "Found {} distros", found);
//! ```
//!
//! [wslapi.h]:     https://docs.microsoft.com/en-us/windows/win32/api/wslapi/

mod configuration;  pub use configuration::*;
mod error;          pub use error::*;
mod flags;          pub use flags::*;
mod library;        pub use library::*;
mod process;        pub use process::*;
pub mod registry;
mod stdio;          pub use stdio::*;
