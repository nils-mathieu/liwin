use super::hwnd::{Hwnd, ShowWindow};
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
        let mut hwnd = Hwnd::new(&config)?;

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
