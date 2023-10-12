use super::hwnd::{Hwnd, ShowWindow, WindowStyles};
use super::wndproc::State;
use super::Error;

/// The [`crate::Window`] implementation for Windows.
pub struct Window {
    state: Box<super::wndproc::State>,
    hwnd: Hwnd,
}

impl Window {
    /// Creates a new [`Window`] instance.
    pub fn new(config: crate::Config) -> Result<Self, Error> {
        let styles = make_window_styles(&config);

        let window_size = match config.size {
            Some((width, height)) => Some(styles.client_to_window_size(width, height)?),
            None => None,
        };

        let mut hwnd = Hwnd::new(
            config.title,
            config.position,
            window_size,
            super::wndproc::wndproc,
        )?;

        // Set the window styles separately as the `CreateWindowExW` function seems
        // to imply some styles.
        hwnd.set_styles(styles)?;

        // Enable the WM_INPUT message.
        hwnd.enable_raw_input()?;

        let mut state: Box<super::wndproc::State> = Box::default();
        hwnd.set_userdata(&mut *state as *mut super::wndproc::State as usize)?;

        Ok(Self { hwnd, state })
    }

    /// See [`crate::Window::set_visible`]
    pub fn set_visible(&mut self, yes: bool) {
        let cmd = if yes {
            ShowWindow::ShowNormal
        } else {
            ShowWindow::Hide
        };

        self.hwnd.show_window(cmd);
    }

    /// See [`crate::Window::client_size`]
    pub fn client_size(&self) -> (u32, u32) {
        let (left, top, right, bottom) = self
            .hwnd
            .get_client_rect()
            .unwrap_or_else(|err| unexpected_windows_error(err));

        ((right - left) as u32, (bottom - top) as u32)
    }

    /// See [`crate::Window::poll_events`]
    pub fn poll_events<F>(&mut self, mut handler: F)
    where
        F: Send + FnMut(crate::Event),
    {
        let guard = HandlerGuard(&mut self.state);
        unsafe { guard.0.set_handler(&mut handler) };
        while self.hwnd.peek_messages() {}
    }

    /// See [`crate::Window::blocking_poll_events`]
    pub fn blocking_poll_events<F>(&mut self, mut handler: F)
    where
        F: Send + FnMut(crate::Event),
    {
        let guard = HandlerGuard(&mut self.state);
        unsafe { guard.0.set_handler(&mut handler) };
        let _ = self.hwnd.get_messages();
        while self.hwnd.peek_messages() {}
    }
}

#[cfg(feature = "raw-window-handle")]
impl rwh::HasWindowHandle for Window {
    #[inline(always)]
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        self.hwnd.window_handle()
    }
}

#[cfg(feature = "raw-window-handle")]
impl rwh::HasDisplayHandle for Window {
    #[inline(always)]
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        self.hwnd.display_handle()
    }
}

/// A guard that automatically removes user-defined handlers when dropped to avoid
/// calling into a dangling function pointer.
struct HandlerGuard<'a>(&'a mut State);

impl<'a> Drop for HandlerGuard<'a> {
    fn drop(&mut self) {
        self.0.remove_handler();
    }
}

/// Converts the window config [`crate::Config`] into the corresponding Windows styles.
///
/// The first element of the tuple is the window style, the second is the extended window style.
fn make_window_styles(config: &crate::Config) -> WindowStyles {
    let mut styles = WindowStyles::empty();

    styles |= WindowStyles::ACCEPT_FILES;

    if config.visible {
        styles |= WindowStyles::VISIBLE;
    }

    if config.always_on_top {
        styles |= WindowStyles::TOPMOST;
    }

    if config.decorations {
        styles |= WindowStyles::CAPTION | WindowStyles::SYSMENU | WindowStyles::MINIMIZE_BOX;

        if config.resizable {
            styles |= WindowStyles::MAXIMIZE_BOX | WindowStyles::SIZE_BOX;
        }
    } else {
        styles |= WindowStyles::BORDER;
    }

    styles
}

/// Windows unexpectedly returned an error.
#[track_caller]
#[cold]
fn unexpected_windows_error(err: Error) -> ! {
    panic!("unexpected windows error: {err:?} - please report this bug!")
}
