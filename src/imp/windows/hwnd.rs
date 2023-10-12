use std::mem::size_of;

use windows_sys::Win32::Foundation::{HMODULE, HWND};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::Error;

/// A wrapper around an [`HWND`].
///
/// It automatically destroys the window upon being dropped.
pub struct Hwnd {
    class: WindowClass,
    hwnd: HWND,
}

impl Hwnd {
    /// Creates a new [`Hwnd`] instance.
    pub fn new(config: &crate::Config) -> Result<Self, Error> {
        let class = WindowClass::new()?;

        let (style, ex_style) = make_window_styles(config);
        let name = make_utf16(config.title);
        let (x, y) = config.position.unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));
        let (w, h) = config
            .size
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
                w,
                h,
                0,
                0,
                class.hinstance,
                std::ptr::null_mut(),
            )
        };

        // Set the window styles separately as the `CreateWindowExW` function seems
        // to imply some styles.
        unsafe {
            SetWindowLongW(hwnd, GWL_STYLE, style as i32);
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style as i32);
        }

        // Only show the window *after* the styles have been applied, and only if it has
        // been requested.
        if config.visible {
            unsafe { ShowWindow(hwnd, SW_SHOWNORMAL) };
        }

        if hwnd == 0 {
            Err(Error::last())
        } else {
            Ok(Self { hwnd, class })
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
    #[inline]
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
    pub fn new() -> Result<Self, Error> {
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
            lpfnWndProc: Some(super::wndproc::wndproc),
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

/// Converts the window config [`crate::Config`] into the corresponding Windows styles.
///
/// The first element of the tuple is the window style, the second is the extended window style.
fn make_window_styles(config: &crate::Config) -> (u32, u32) {
    let mut style = 0;
    let mut ex_style = 0;

    ex_style |= WS_EX_ACCEPTFILES;

    if config.visible {
        style |= WS_VISIBLE;
    }

    if config.always_on_top {
        ex_style |= WS_EX_TOPMOST;
    }

    if config.decorations {
        style |= WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;

        if config.resizable {
            style |= WS_SIZEBOX;
            style |= WS_MAXIMIZEBOX;
        }
    } else {
        style |= WS_POPUP;
    }

    (style, ex_style)
}

/// Creates a null-terminated UTF-16 string from the given Rust string.
fn make_utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
