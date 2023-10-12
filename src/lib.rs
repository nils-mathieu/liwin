//! The [`liwin`](self) crate provides a platform-agnostic interface to interact with the
//! windowing system of the operating system.

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod config;
mod error;
mod event;
mod window;

pub use self::config::*;
pub use self::error::*;
pub use self::event::*;
pub use self::window::*;

#[cfg_attr(target_os = "windows", path = "imp/windows/mod.rs")]
mod imp;
