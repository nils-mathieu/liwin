use std::fmt;

use crate::imp;

/// An event received from the windowing system.
pub enum Event {
    /// The user requested the window to close itself.
    ///
    /// This event is usually triggered by the user clicking on the close button of the window.
    ///
    /// # Note
    ///
    /// The window won't be closed automatically. It is the responsibility of the application to
    /// close the window (or not close it at all) when the event is received.
    CloseRequested,

    /// The window has been resized.
    ///
    /// If the window is now minimized, the new width and height of the window will be 0.
    Resized {
        /// The new width of the window, in pixels.
        width: u32,
        /// The new height of the window, in pixels.
        height: u32,
    },

    /// The window has been moved.
    ///
    /// If the window is now minimized, it is unspecified whether a [`Moved`] event will be
    /// generated. If it is, the new X and Y position of the window is unspecified as well.
    ///
    /// [`Moved`]: Event::Moved
    Moved {
        /// The new X position of the window, in pixels.
        x: i32,
        /// The new Y position of the window, in pixels.
        y: i32,
    },

    /// The cursor has been moved over the window.
    ///
    /// This event is only generated when the cursor is over the window. It cannot be used to track
    /// the mouse position outside of the window.
    CursorMoved {
        /// The new X position of the cursor, in pixels.
        x: i32,
        /// The new Y position of the cursor, in pixels.
        y: i32,
    },

    /// A character has been entered.
    Text(char),

    /// A mouse has been moved.
    ///
    /// Note that this event cannot be used to determine the position of the cursor. It is not
    /// subject to system-specific cursor acceleration and sensitivity settings.
    MouseMoved {
        /// The device that generated the event.
        device: Device,
        /// The horizontal delta of the mouse movement.
        dx: f64,
        /// The vertical delta of the mouse movement.
        dy: f64,
    },

    /// A mouse wheel has been rotated.
    MouseWheel {
        /// The device that generated the event.
        device: Device,
        /// The horizontal delta of the mouse wheel.
        dx: f64,
        /// The vertical delta of the mouse wheel.
        dy: f64,
    },

    /// A mouse button has been pressed or released.
    MouseButton {
        /// The device that generated the event.
        device: Device,
        /// The button that has been pressed or released.
        button: MouseButton,
        /// Whether the button is now pressed.
        ///
        /// If `true`, the button is now pressed. If `false`, the button is now released.
        pressed: bool,
    },

    /// A keyboard key has been pressed or released.
    KeyboardKey {
        /// The device that generated the event.
        device: Device,
        /// The key that has been pressed or released.
        ///
        /// Note the [`Option<T>`]. Not all keyboard key events have an associated logical key.
        key: Option<Key>,
        /// A unique code that identifies the key that has been pressed or released.
        code: KeyCode,
        /// Whether the key is now pressed.
        ///
        /// If `true`, the key is now pressed. If `false`, the key is now released.
        pressed: bool,
    },
}

/// An external human interface device.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Device(pub(crate) imp::Device);

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// A button that can either be pressed or released.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButton(pub u8);

impl MouseButton {
    /// The left mouse button.
    pub const LEFT: Self = Self(0);

    /// The middle mouse button.
    pub const MIDDLE: Self = Self(1);

    /// The right mouse button.
    pub const RIGHT: Self = Self(2);
}

impl fmt::Debug for MouseButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::LEFT => f.pad("LEFT"),
            Self::MIDDLE => f.pad("MIDDLE"),
            Self::RIGHT => f.pad("RIGHT"),
            Self(num) => f.debug_tuple("MouseButton").field(&num).finish(),
        }
    }
}

/// A keyboard key.
///
/// This enumeration is a layout-independent representation of a keyboard key. It represents their
/// logical meaning rather than their physical location on the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// The **BACKSPACE** key.
    Backspace = 1,
    /// The **TAB** key.
    Tab,
    /// The **ENTER** key.
    Enter,
    /// The **ESCAPE** key.
    Escape,
    /// The **SPACE** key.
    Space,
    /// The **PAGE UP** key.
    PageUp,
    /// The **PAGE DOWN** key.
    PageDown,
    /// The **END** key.
    End,
    /// The **HOME** key.
    Home,
    /// The **LEFT** arrow key.
    Left,
    /// The **UP** arrow key.
    Up,
    /// The **RIGHT** arrow key.
    Right,
    /// The **DOWN** arrow key.
    Down,
    /// The **INSERT** key.
    Insert,
    /// The **DELETE** key.
    Delete,
    /// The **0** key, over the letters.
    Zero,
    /// The **1** key, over the letters.
    One,
    /// The **2** key, over the letters.
    Two,
    /// The **3** key, over the letters.
    Three,
    /// The **4** key, over the letters.
    Four,
    /// The **5** key, over the letters.
    Five,
    /// The **6** key, over the letters.
    Six,
    /// The **7** key, over the letters.
    Seven,
    /// The **8** key, over the letters.
    Eight,
    /// The **9** key, over the letters.
    Nine,
    /// The **A** key.
    A,
    /// The **B** key.
    B,
    /// The **C** key.
    C,
    /// The **D** key.
    D,
    /// The **E** key.
    E,
    /// The **F** key.
    F,
    /// The **G** key.
    G,
    /// The **H** key.
    H,
    /// The **I** key.
    I,
    /// The **J** key.
    J,
    /// The **K** key.
    K,
    /// The **L** key.
    L,
    /// The **M** key.
    M,
    /// The **N** key.
    N,
    /// The **O** key.
    O,
    /// The **P** key.
    P,
    /// The **Q** key.
    Q,
    /// The **R** key.
    R,
    /// The **S** key.
    S,
    /// The **T** key.
    T,
    /// The **U** key.
    U,
    /// The **V** key.
    V,
    /// The **W** key.
    W,
    /// The **X** key.
    X,
    /// The **Y** key.
    Y,
    /// The **Z** key.
    Z,
    /// The **F1** key.
    F1,
    /// The **F2** key.
    F2,
    /// The **F3** key.
    F3,
    /// The **F4** key.
    F4,
    /// The **F5** key.
    F5,
    /// The **F6** key.
    F6,
    /// The **F7** key.
    F7,
    /// The **F8** key.
    F8,
    /// The **F9** key.
    F9,
    /// The **F10** key.
    F10,
    /// The **F11** key.
    F11,
    /// The **F12** key.
    F12,
    /// The **F13** key.
    F13,
    /// The **F14** key.
    F14,
    /// The **F15** key.
    F15,
    /// The **F16** key.
    F16,
    /// The **F17** key.
    F17,
    /// The **F18** key.
    F18,
    /// The **F19** key.
    F19,
    /// The **F20** key.
    F20,
    /// The **F21** key.
    F21,
    /// The **F22** key.
    F22,
    /// The **F23** key.
    F23,
    /// The **F24** key.
    F24,
    /// The **NUM LOCK** key.
    NumLock,
    /// The **SCROLL LOCK** key.
    ScrollLock,
    /// The left **SHIFT** key.
    LeftShift,
    /// The right **SHIFT** key.
    RightShift,
    /// The left **CONTROL** key.
    LeftControl,
    /// The right **CONTROL** key.
    RightControl,
    /// The left **ALT** key.
    LeftAlt,
    /// The right **ALT** key.
    RightAlt,
    /// The left **META** key.
    LeftMeta,
    /// The right **META** key.
    RightMeta,
    /// The **MENU** key.
    Menu,
    /// The **PRINT SCREEN** key.
    PrintScreen,
    /// The **PAUSE** key.
    Pause,
    /// The **CAPS LOCK** key.
    CapsLock,
    /// The **UP** volume key.
    VolumeUp,
    /// The **DOWN** volume key.
    VolumeDown,
    /// The **MUTE** volume key.
    VolumeMute,
    /// The **PLAY/PAUSE** media key.
    MediaPlayPause,
    /// The **STOP** media key.
    MediaStop,
    /// The **PREVIOUS** media key.
    MediaPrevious,
    /// The **NEXT** media key.
    MediaNext,
    /// The **0** numpad key.
    Numpad0,
    /// The **1** numpad key.
    Numpad1,
    /// The **2** numpad key.
    Numpad2,
    /// The **3** numpad key.
    Numpad3,
    /// The **4** numpad key.
    Numpad4,
    /// The **5** numpad key.
    Numpad5,
    /// The **6** numpad key.
    Numpad6,
    /// The **7** numpad key.
    Numpad7,
    /// The **8** numpad key.
    Numpad8,
    /// The **9** numpad key.
    Numpad9,
    /// The **.** numpad key.
    NumpadDecimal,
    /// The **+** numpad key.
    NumpadAdd,
    /// The **-** numpad key.
    NumpadSubtract,
    /// The **\*** numpad key.
    NumpadMultiply,
    /// The **/** numpad key.
    NumpadDivide,
    /// The **ENTER** numpad key.
    NumpadEnter,
}

/// A unique code that identifies a keyboard key.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCode(pub(crate) imp::KeyCode);

impl KeyCode {
    /// Creates a new [`KeyCode`] with the given code.
    pub const fn from_code(code: u8) -> Self {
        Self(imp::KeyCode::from_code(code))
    }
}

impl fmt::Debug for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
