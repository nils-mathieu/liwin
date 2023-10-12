use std::mem::size_of;

use bitflags::bitflags;
use windows_sys::Win32::Foundation::{HMODULE, HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::Error;

/// The function signature of the window procedure.
pub type WndprocFn = unsafe extern "system" fn(HWND, u32, usize, isize) -> isize;

/// A wrapper around an [`HWND`].
///
/// It automatically destroys the window upon being dropped.
pub struct Hwnd {
    class: WindowClass,
    hwnd: HWND,
}

impl Hwnd {
    /// Creates a new [`Hwnd`] instance.
    ///
    /// # Notes
    ///
    /// This function sets the window styles to 0 because the `CreateWindowExW` function implies
    /// some styles which may not be desirable. Instead, use [`Hwnd::set_styles`] to set the
    /// styles of the window.
    pub fn new(
        title: &str,
        position: Option<(i32, i32)>,
        size: Option<(u32, u32)>,
        wndproc: WndprocFn,
    ) -> Result<Self, Error> {
        let class = WindowClass::new(wndproc)?;

        let name = make_utf16(title);
        let (x, y) = position.unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));
        let (width, height) = size
            .map(|(w, h)| (w as i32, h as i32))
            .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.atom as *const u16,
                name.as_ptr(),
                0,
                x,
                y,
                width,
                height,
                0,
                0,
                class.hinstance,
                std::ptr::null_mut(),
            )
        };

        if hwnd == 0 {
            Err(Error::last())
        } else {
            Ok(Self { hwnd, class })
        }
    }

    /// Sets the style of the window.
    pub fn set_styles(&mut self, style: WindowStyles) -> Result<(), Error> {
        unsafe {
            let (style, ex_style) = style.to_raw_styles();

            // Set the EX_STYLE first to ensure that if the STYLE has a VISIBLE flag, the window
            // will be shown with the correct styles.
            let a = set_window_long(self.hwnd, GWL_EXSTYLE, ex_style as i32);
            let b = set_window_long(self.hwnd, GWL_STYLE, style as i32);

            a.and(b)
        }
    }

    /// Sets the window's show state.
    ///
    /// # Returns
    ///
    /// - `true` if the window was previously visible.
    ///
    /// - `false` if the window was previously hidden.
    #[inline]
    pub fn show_window(&mut self, cmd: ShowWindow) -> bool {
        let ret = unsafe { ShowWindow(self.hwnd, cmd as SHOW_WINDOW_CMD) };
        ret != 0
    }

    /// Enables raw input for the window for the mouse and keyboard.
    pub fn enable_raw_input(&mut self) -> Result<(), Error> {
        use windows_sys::Win32::Devices::HumanInterfaceDevice::*;
        use windows_sys::Win32::UI::Input::*;

        unsafe {
            let devices = [
                RAWINPUTDEVICE {
                    usUsagePage: HID_USAGE_PAGE_GENERIC,
                    usUsage: HID_USAGE_GENERIC_KEYBOARD,
                    dwFlags: 0,
                    hwndTarget: self.hwnd,
                },
                RAWINPUTDEVICE {
                    usUsagePage: HID_USAGE_PAGE_GENERIC,
                    usUsage: HID_USAGE_GENERIC_MOUSE,
                    dwFlags: 0,
                    hwndTarget: self.hwnd,
                },
            ];

            let ret = RegisterRawInputDevices(
                devices.as_ptr(),
                devices.len() as u32,
                size_of::<RAWINPUTDEVICE>() as u32,
            );

            if ret == 0 {
                Err(Error::last())
            } else {
                Ok(())
            }
        }
    }

    /// Sets the userdata associated with the window.
    pub fn set_userdata(&mut self, userdata: usize) -> Result<(), Error> {
        Error::SUCCESS.make_last();

        let ret = unsafe { SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, userdata as isize) };

        if ret == 0 {
            let err = Error::last();
            if err != Error::SUCCESS {
                return Err(err);
            }
        }

        Ok(())
    }

    /// Gets the rectangle of the client area of the window.
    ///
    /// # Returns
    ///
    /// On success, the bounds of the client area of the window are returned in the following
    /// order: `(left, top, right, bottom)`.
    pub fn get_client_rect(&self) -> Result<(i32, i32, i32, i32), Error> {
        let mut rect = unsafe { std::mem::zeroed() };
        let ret = unsafe { GetClientRect(self.hwnd, &mut rect) };

        if ret == 0 {
            Err(Error::last())
        } else {
            Ok((rect.left, rect.top, rect.right, rect.bottom))
        }
    }

    /// Executes the message handler of the window, calling it for *potentially* multiple messages,
    /// but not necessarily all.
    pub fn get_messages(&mut self) -> Result<(), Error> {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            let ret = GetMessageW(&mut msg, self.hwnd, 0, 0);

            match ret {
                -1 => Err(Error::last()),
                0 => Ok(()),
                _ => {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);

                    Ok(())
                }
            }
        }
    }

    /// Executes the message handler of the window, calling it for *potentially* multiple messages,
    /// but not necessarily all.
    ///
    /// This function does not block, and returns `true` if a message was handled.
    pub fn peek_messages(&mut self) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;

        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            let ret = PeekMessageW(&mut msg, self.hwnd, 0, 0, PM_REMOVE);

            if ret == 0 {
                false
            } else {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                true
            }
        }
    }
}

impl Drop for Hwnd {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) };
    }
}

/// The different ways to show (or hide) a window.
#[repr(u32)]
pub enum ShowWindow {
    /// The window should be hidden. Another window will take the focus.
    Hide = SW_HIDE,
    /// Show the window normally, restoring an eventual previous state.
    ShowNormal = SW_SHOWNORMAL,
}

#[cfg(feature = "raw-window-handle")]
impl rwh::HasWindowHandle for Hwnd {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        use std::num::NonZeroIsize;

        let hwnd = unsafe { NonZeroIsize::new_unchecked(self.hwnd) };
        let hinstance = unsafe { NonZeroIsize::new_unchecked(self.class.hinstance) };

        let mut raw = rwh::Win32WindowHandle::new(hwnd);
        raw.hinstance = Some(hinstance);

        // SAFETY:
        //  The `Hwnd` type guarantees that its inner window object remain valid for its own
        //  lifetime.
        let handle = unsafe { rwh::WindowHandle::borrow_raw(raw.into()) };

        Ok(handle)
    }
}

#[cfg(feature = "raw-window-handle")]
impl rwh::HasDisplayHandle for Hwnd {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let raw = rwh::WindowsDisplayHandle::new();

        // SAFETY:
        //  See the safety note in the `HasWindowHandle` implementation.
        let handle = unsafe { rwh::DisplayHandle::borrow_raw(raw.into()) };

        Ok(handle)
    }
}

/// Represents a window class.
///
/// This type is mostly a way to have a destructor that unregisters the window class automatically.
struct WindowClass {
    hinstance: HMODULE,
    atom: u16,
}

impl WindowClass {
    /// Creates a new [`WindowClass`] instance.
    pub fn new(wndproc: WndprocFn) -> Result<Self, Error> {
        let hinstance = get_current_hinstance()?;

        let info = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hCursor: 0,
            hIcon: 0,
            hInstance: hinstance,
            hbrBackground: 0,
            style: CS_HREDRAW | CS_VREDRAW,
            lpszMenuName: std::ptr::null(),
            lpszClassName: windows_sys::w!("liwin_window_class"),
            lpfnWndProc: Some(wndproc),
            hIconSm: 0,
        };

        let atom = unsafe { RegisterClassExW(&info) };

        if atom == 0 {
            Err(Error::last())
        } else {
            Ok(Self { hinstance, atom })
        }
    }
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        unsafe { UnregisterClassW(self.atom as *const u16, self.hinstance) };
    }
}

/// Returns the `HMODULE` handle of the current executable.
fn get_current_hinstance() -> Result<HMODULE, Error> {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

    // I don't think `GetModuleHandleW` can fail when it is given a null, but the documentation
    // for it is not clear on this so we'll check anyway.
    let ret = unsafe { GetModuleHandleW(std::ptr::null()) };

    if ret == 0 {
        Err(Error::last())
    } else {
        Ok(ret)
    }
}

bitflags! {
    /// A set of window styles.
    ///
    /// # Representation
    ///
    /// The lower 32 bits represent the window style, the higher 32 bits represent the extended
    /// window style.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WindowStyles: u64 {
        /// The window should be visible.
        const VISIBLE = WS_VISIBLE as u64;

        /// The window should always appear on top of other windows.
        const TOPMOST = (WS_EX_TOPMOST as u64) >> 32;

        /// The window should have a title bar.
        ///
        /// This includes the [`BORDER`](WindowStyles::BORDER) style.
        const CAPTION = (WS_CAPTION | WS_BORDER) as u64;

        /// The window should have the default system menu.
        const SYSMENU = WS_SYSMENU as u64;

        /// The window has a thin border.
        const BORDER = WS_POPUP as u64;

        /// The window has a sizing border.
        const SIZE_BOX = WS_SIZEBOX as u64;

        /// The window has a minimize box.
        const MAXIMIZE_BOX = WS_MAXIMIZEBOX as u64;

        /// The window has a minimize box.
        const MINIMIZE_BOX = WS_MINIMIZEBOX as u64;

        /// The window accepts dropped files.
        const ACCEPT_FILES = (WS_EX_ACCEPTFILES as u64) >> 32;
    }
}

impl WindowStyles {
    /// Converts this [`WindowStyles`] instance into the corresponding raw styles.
    fn to_raw_styles(self) -> (WINDOW_STYLE, WINDOW_EX_STYLE) {
        let bits = self.bits();

        let style = bits as u32;
        let ex_style = (bits >> 32) as u32;

        (style, ex_style)
    }

    /// Converts the given client size to the corresponding window size, for a window with the
    /// provided styles.
    pub fn client_to_window_rect(
        self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<(i32, i32, i32, i32), Error> {
        let (style, ex_style) = self.to_raw_styles();

        let mut rect = RECT {
            left,
            top,
            right,
            bottom,
        };

        let ret = unsafe { AdjustWindowRectEx(&mut rect, style, 0, ex_style) };

        if ret == 0 {
            Err(Error::last())
        } else {
            Ok((rect.left, rect.top, rect.right, rect.bottom))
        }
    }

    /// Converts the given client size to the corresponding window size, for a window with the
    /// provided styles.
    pub fn client_to_window_size(self, width: u32, height: u32) -> Result<(u32, u32), Error> {
        let (left, top, right, bottom) =
            self.client_to_window_rect(0, 0, width as i32, height as i32)?;
        Ok(((right - left) as u32, (bottom - top) as u32))
    }
}

/// Creates a null-terminated UTF-16 string from the given Rust string.
fn make_utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Wraps the `SetWindowLongW` function and returns a standard result.
///
/// # Safety
///
/// No idea, check the [documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowlongw).
unsafe fn set_window_long(
    hwnd: HWND,
    index: WINDOW_LONG_PTR_INDEX,
    value: i32,
) -> Result<(), Error> {
    Error::SUCCESS.make_last();

    let ret = unsafe { SetWindowLongW(hwnd, index, value) };

    if ret == 0 {
        let err = Error::last();
        if err != Error::SUCCESS {
            return Err(err);
        }
    }

    Ok(())
}
