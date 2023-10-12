/// The configuration of a window.
#[derive(Debug, Clone)]
pub struct Config<'a> {
    /// The title of the window.
    ///
    /// **Default:** `"My Awesome Window"`
    pub title: &'a str,

    /// The initial position of the window.
    ///
    /// If `None`, a platform-specific default position will be used instead.
    ///
    /// **Default:** `None`
    pub position: Option<(i32, i32)>,

    /// The initial size of the window.
    ///
    /// If `None`, a platform-specific default size will be used instead.
    ///
    /// **Default:** `None`
    pub size: Option<(u32, u32)>,

    /// The window should be initially visible.
    ///
    /// **Default:** `true`
    pub visible: bool,

    /// Whether the window should be resizable.
    ///
    /// **Default:** `true`
    pub resizable: bool,

    /// Whether the window should always appear on top of other windows.
    ///
    /// **Default:** `false`
    pub always_on_top: bool,

    /// Whether the window should include the system's default decorations.
    ///
    /// **Default:** `true`
    pub decorations: bool,
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        Self {
            title: "My Awesome Window",
            position: None,
            size: None,
            visible: true,
            resizable: true,
            always_on_top: false,
            decorations: true,
        }
    }
}
