//! The implementation of the [`liwin`](crate) crate for Windows.

mod error;
mod window;

pub use self::error::*;
pub use self::window::*;

mod hwnd;
mod wndproc;

/// The type that uniquely identifies a device.
pub type Device = windows_sys::Win32::Foundation::HANDLE;

/// The type that uniquely identifies a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCode {
    /// The make-code of the key.
    pub code: u16,
    /// Some flags that are set when the key is extended.
    ///
    /// Specifically, the following flags are defined:
    ///
    /// - `0x01`: The key has an E0 prefix.
    /// - `0x02`: The key has an E1 prefix.
    pub extended_flags: u8,
}

impl KeyCode {
    /// See [`crate::KeyCode::from_code`].
    pub const fn from_code(code: u8) -> Self {
        Self {
            code: code as u16,
            extended_flags: 0,
        }
    }
}
