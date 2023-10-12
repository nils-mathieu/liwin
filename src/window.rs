use crate::{imp, Error, Event};

/// Represents a window.
///
/// This handle may be used to interact with the windowing system and the window itself.
pub struct Window(pub(crate) imp::Window);

impl Window {
    /// Creates a new [`Window`] instance, initiating a connection to the windowing system.
    pub fn new(config: &crate::Config) -> Result<Self, Error> {
        match imp::Window::new(config) {
            Ok(window) => Ok(Self(window)),
            Err(error) => Err(Error(error)),
        }
    }

    /// Sets the visibility of the window.
    pub fn set_visible(&mut self, yes: bool) {
        self.0.set_visible(yes);
    }

    /// Calls the given closure with the new, unprocessed events.
    ///
    /// If no events are available, this function will return immediately.
    #[inline(always)]
    pub fn poll_events(&mut self, handler: impl FnMut(Event)) {
        self.0.poll_events(handler);
    }

    /// Calls the given closure with the new, unprocessed events.
    ///
    /// If no events are available, this function will block until one is received.
    #[inline(always)]
    pub fn blocking_poll_events(&mut self, handler: impl FnMut(Event)) {
        self.0.blocking_poll_events(handler);
    }
}

#[cfg(feature = "raw-window-handle")]
impl rwh::HasWindowHandle for Window {
    #[inline(always)]
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        self.0.window_handle()
    }
}

#[cfg(feature = "raw-window-handle")]
impl rwh::HasDisplayHandle for Window {
    #[inline(always)]
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        self.0.display_handle()
    }
}
