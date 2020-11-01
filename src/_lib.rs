#![cfg(windows)]
#![deny(missing_docs)]

//! APIs for managing the Windows Subsystem for Linux ([wslapi.h])
//!
//! ```rust
//! use wslapi::*;
//! 
//! let ubuntu = "Ubuntu";
//! let nonexistant = "Nonexistant";
//! 
//! let wsl = Library::new().unwrap();
//! assert!( wsl.is_distribution_registered(ubuntu));
//! assert!(!wsl.is_distribution_registered(nonexistant));
//! assert!(wsl.get_distribution_configuration(nonexistant).is_err());
//! 
//! let cfg = wsl.get_distribution_configuration(ubuntu).unwrap();
//! assert!(cfg.default_uid == 0 || (1000 ..= 2000).contains(&cfg.default_uid));
//! // 0 == root, 1000+ == regular user
//! assert!(cfg.flags & WSL_DISTRIBUTION_FLAGS::DEFAULT == WSL_DISTRIBUTION_FLAGS::DEFAULT);
//! // `cfg.flags` contains extra, undocumented flags like 0x8
//! assert!((1..2).contains(&cfg.version)); // WSL version
//!
//! wsl.launch_interactive(ubuntu, "echo testing 123", true).unwrap();
//! 
//! let stdin  = "echo testing 456\necho PATH: ${PATH}\n";
//! let stdout = std::fs::File::create("target/basic.txt").unwrap();
//! let stderr = (); // shorthand for Stdio::null()
//! wsl.launch(ubuntu, "sh", true, stdin, stdout, stderr).unwrap().wait().unwrap();
//! ```
//!
//! [wslapi.h]:     https://docs.microsoft.com/en-us/windows/win32/api/wslapi/

mod configuration;  pub use configuration::*;
mod error;          pub use error::*;
mod flags;          pub use flags::*;
mod library;        pub use library::*;
mod process;        pub use process::*;
mod stdio;          pub use stdio::*;
