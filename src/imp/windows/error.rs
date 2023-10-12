use std::fmt;

use windows_sys::Win32::Foundation::{GetLastError, SetLastError, ERROR_SUCCESS, WIN32_ERROR};

/// The error type on the Windows platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Error(pub WIN32_ERROR);

impl Error {
    /// The success error code.
    pub const SUCCESS: Self = Self(ERROR_SUCCESS);

    /// Returns the last error code that occured on the current thread.
    #[inline]
    pub fn last() -> Self {
        unsafe { Self(GetLastError()) }
    }

    /// Makes this error the last error on the current thread.
    #[inline]
    pub fn make_last(self) {
        unsafe { SetLastError(self.0) };
    }

    /// Returns the message associated with the error, as a raw UTF-16 string.
    pub fn read_message(self, buf: &mut [u16]) -> Result<usize, Error> {
        use windows_sys::Win32::System::Diagnostics::Debug::*;

        let len = unsafe {
            FormatMessageW(
                FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
                std::ptr::null(),
                self.0,
                0,
                buf.as_mut_ptr(),
                buf.len() as u32,
                std::ptr::null_mut(),
            )
        };

        if len == 0 {
            Err(Error::last())
        } else {
            Ok(len as usize)
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let mut buf = Box::new([0u16; 1024]);
        let len = self.read_message(buf.as_mut()).map_err(|_| fmt::Error)?;
        let message = OsString::from_wide(&buf[..len]);
        let s = message.to_str().ok_or(fmt::Error)?;

        f.pad(s)
    }
}

impl std::error::Error for Error {}
