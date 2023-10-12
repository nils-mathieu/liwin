use std::mem::size_of;

use windows_sys::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::KeyCode;

/// The default window procedure for windows created by this crate.
///
/// This basically always just calls `DefWindowProcW`, but notably ignores the `WM_CLOSE` message.
fn default_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => 0,
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// The default handler function for [`State`].
#[allow(unused_variables)]
fn default_state_handler(state: *mut (), event: crate::Event) {}

/// Stores some state that is required to transform window events into [`crate::Event`]s.
pub struct State {
    /// A function to call when the window receives an event.
    ///
    /// # Safety
    ///
    /// This function expects the `state` parameter to be a pointer to the associated
    /// `handler_state`.
    handler_fn: unsafe fn(state: *mut (), event: crate::Event),
    /// The state to be passed to the `handler_fn`.
    handler_state: *mut (),

    /// An eventual low surrogate of a UTF-16 character.
    ///
    /// This is kept until the next `WM_CHAR` message is received, at which point it is combined
    /// with the high surrogate to form a full UTF-32 code point.
    ///
    /// 0 means that no surrogate is stored.
    low_surrogate: u16,
}

impl State {
    /// Sets the handler function.
    ///
    /// # Safety
    ///
    /// The handler function must be removed before it becomes invalid.
    #[inline]
    pub unsafe fn set_handler<F>(&mut self, handler: &mut F)
    where
        F: FnMut(crate::Event),
    {
        fn handler_fn<F>(state: *mut (), event: crate::Event)
        where
            F: FnMut(crate::Event),
        {
            let f = unsafe { &mut *(state as *mut F) };
            f(event);
        }

        self.handler_fn = handler_fn::<F>;
        self.handler_state = handler as *mut F as *mut ();
    }

    /// Removes the handler function.
    #[inline]
    pub fn remove_handler(&mut self) {
        self.handler_fn = default_state_handler;
    }

    /// Sends an event to the handler function.
    #[inline]
    pub fn send_event(&mut self, event: crate::Event) {
        unsafe { (self.handler_fn)(self.handler_state, event) };
    }

    /// Take an UTF-16 code point.
    pub fn take_u16_code_point(&mut self, code: u16) {
        if is_low_surrogate(code) {
            self.low_surrogate = code;
        } else if is_high_surrogate(code) {
            if self.low_surrogate == 0 {
                return;
            }

            if let Some(c) = decode_utf16(self.low_surrogate, code) {
                self.send_event(crate::Event::Text(c));
            }

            self.low_surrogate = 0;
        } else if let Some(c) = char::from_u32(code as u32) {
            self.send_event(crate::Event::Text(c));

            self.low_surrogate = 0;
        }
    }
}

impl Default for State {
    #[inline]
    fn default() -> Self {
        Self {
            handler_fn: default_state_handler,
            handler_state: std::ptr::null_mut(),
            low_surrogate: 0,
        }
    }
}

/// Gets a pointer to an instance of `T` in the userdata field of the provided window.
///
/// # Safety
///
/// - `hwnd` must be a valid window handle.
///
/// - The userdata field of the window must be either 0, or a valid pointer to an instance of `T`
///   that will live for the lifetime `'a`.
unsafe fn get_handler_in_userdata<'a, T>(hwnd: HWND) -> Option<&'a mut T> {
    unsafe {
        let userdata = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut T;

        if userdata.is_null() {
            None
        } else {
            Some(&mut *userdata)
        }
    }
}

/// The window procedure for windows created by this crate.
///
/// This function expects the USERDATA field of the window to be either 0, or a pointer to an
/// instance of `F`.
pub unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let Some(state) = get_handler_in_userdata::<State>(hwnd) else {
            return default_wndproc(hwnd, msg, wparam, lparam);
        };

        match msg {
            WM_CLOSE => {
                state.send_event(crate::Event::CloseRequested);
            }
            WM_SIZE => {
                let width = lparam as u16 as u32;
                let height = (lparam >> 16) as u16 as u32;
                state.send_event(crate::Event::Resized { width, height });
            }
            WM_MOVE => {
                let x = lparam as i16 as i32;
                let y = (lparam >> 16) as i16 as i32;
                state.send_event(crate::Event::Moved { x, y });
            }
            WM_MOUSEMOVE => {
                let x = lparam as i16 as i32;
                let y = (lparam >> 16) as i16 as i32;
                state.send_event(crate::Event::CursorMoved { x, y });
            }
            WM_INPUT => {
                handle_raw_input(lparam as HRAWINPUT, state);
            }
            WM_CHAR => {
                state.take_u16_code_point(wparam as u16);
            }
            _ => (),
        }

        default_wndproc(hwnd, msg, wparam, lparam)
    }
}

/// Handles a raw input event, eventually converting it to a [`crate::Event`].
fn handle_raw_input(handle: HRAWINPUT, state: &mut State) {
    let Some(rawinput) = read_rawinput(handle) else {
        return;
    };

    match rawinput.header.dwType {
        RIM_TYPEKEYBOARD => {
            let keyboard = unsafe { &rawinput.data.keyboard };
            handle_keyboard_event(rawinput.header.hDevice, keyboard, state);
        }
        RIM_TYPEMOUSE => {
            let mouse = unsafe { &rawinput.data.mouse };
            handle_mouse_event(rawinput.header.hDevice, mouse, state);
        }
        _ => (),
    }
}

/// Reads a [`RAWINPUT`] from the given handle.
fn read_rawinput(handle: HRAWINPUT) -> Option<RAWINPUT> {
    unsafe {
        let mut rawinput: RAWINPUT = std::mem::zeroed();
        let mut rawinput_size = size_of::<RAWINPUT>() as u32;

        let ret = GetRawInputData(
            handle,
            RID_INPUT,
            &mut rawinput as *mut RAWINPUT as _,
            &mut rawinput_size,
            size_of::<RAWINPUTHEADER>() as u32,
        );

        if ret == u32::MAX {
            None
        } else {
            Some(rawinput)
        }
    }
}

/// Handles a raw keyboard event.
fn handle_keyboard_event(device: HANDLE, keyboard: &RAWKEYBOARD, state: &mut State) {
    // Skip "fake" keys.
    if keyboard.VKey == 255 {
        return;
    }

    state.send_event(crate::Event::KeyboardKey {
        device: crate::Device(device),
        code: crate::KeyCode(make_keycode(keyboard)),
        pressed: (keyboard.Flags as u32 & RI_KEY_BREAK) == RI_KEY_MAKE,
        key: compute_key(keyboard),
    });
}

/// Computes the [`crate::Key`] associated with the provided event.
fn compute_key(event: &RAWKEYBOARD) -> Option<crate::Key> {
    // This this for the details:
    //
    //     https://blog.molecular-matters.com/2011/09/05/properly-handling-keyboard-input/
    //
    // I wonder how Microsoft ended up with this mess.

    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

    let e0 = (event.Flags as u32 & RI_KEY_E0) != 0;

    match event.VKey {
        VK_BACK => Some(crate::Key::Backspace),
        VK_TAB => Some(crate::Key::Tab),
        VK_RETURN => {
            if e0 {
                Some(crate::Key::NumpadEnter)
            } else {
                Some(crate::Key::Enter)
            }
        }
        VK_ESCAPE => Some(crate::Key::Escape),
        VK_SPACE => Some(crate::Key::Space),
        VK_PRIOR => {
            if e0 {
                Some(crate::Key::PageUp)
            } else {
                Some(crate::Key::Numpad9)
            }
        }
        VK_NEXT => {
            if e0 {
                Some(crate::Key::PageDown)
            } else {
                Some(crate::Key::Numpad3)
            }
        }
        VK_END => {
            if e0 {
                Some(crate::Key::End)
            } else {
                Some(crate::Key::Numpad1)
            }
        }
        VK_HOME => {
            if e0 {
                Some(crate::Key::Home)
            } else {
                Some(crate::Key::Numpad7)
            }
        }
        VK_LEFT => {
            if e0 {
                Some(crate::Key::Left)
            } else {
                Some(crate::Key::Numpad4)
            }
        }
        VK_UP => {
            if e0 {
                Some(crate::Key::Up)
            } else {
                Some(crate::Key::Numpad8)
            }
        }
        VK_RIGHT => {
            if e0 {
                Some(crate::Key::Right)
            } else {
                Some(crate::Key::Numpad6)
            }
        }
        VK_DOWN => {
            if e0 {
                Some(crate::Key::Down)
            } else {
                Some(crate::Key::Numpad2)
            }
        }
        VK_INSERT => {
            if e0 {
                Some(crate::Key::Insert)
            } else {
                Some(crate::Key::Numpad0)
            }
        }
        VK_DELETE => {
            if e0 {
                Some(crate::Key::Delete)
            } else {
                Some(crate::Key::NumpadDecimal)
            }
        }
        48 => Some(crate::Key::Zero),
        49 => Some(crate::Key::One),
        50 => Some(crate::Key::Two),
        51 => Some(crate::Key::Three),
        52 => Some(crate::Key::Four),
        53 => Some(crate::Key::Five),
        54 => Some(crate::Key::Six),
        55 => Some(crate::Key::Seven),
        56 => Some(crate::Key::Eight),
        57 => Some(crate::Key::Nine),
        65 => Some(crate::Key::A),
        66 => Some(crate::Key::B),
        67 => Some(crate::Key::C),
        68 => Some(crate::Key::D),
        69 => Some(crate::Key::E),
        70 => Some(crate::Key::F),
        71 => Some(crate::Key::G),
        72 => Some(crate::Key::H),
        73 => Some(crate::Key::I),
        74 => Some(crate::Key::J),
        75 => Some(crate::Key::K),
        76 => Some(crate::Key::L),
        77 => Some(crate::Key::M),
        78 => Some(crate::Key::N),
        79 => Some(crate::Key::O),
        80 => Some(crate::Key::P),
        81 => Some(crate::Key::Q),
        82 => Some(crate::Key::R),
        83 => Some(crate::Key::S),
        84 => Some(crate::Key::T),
        85 => Some(crate::Key::U),
        86 => Some(crate::Key::V),
        87 => Some(crate::Key::W),
        88 => Some(crate::Key::X),
        89 => Some(crate::Key::Y),
        90 => Some(crate::Key::Z),
        VK_F1 => Some(crate::Key::F1),
        VK_F2 => Some(crate::Key::F2),
        VK_F3 => Some(crate::Key::F3),
        VK_F4 => Some(crate::Key::F4),
        VK_F5 => Some(crate::Key::F5),
        VK_F6 => Some(crate::Key::F6),
        VK_F7 => Some(crate::Key::F7),
        VK_F8 => Some(crate::Key::F8),
        VK_F9 => Some(crate::Key::F9),
        VK_F10 => Some(crate::Key::F10),
        VK_F11 => Some(crate::Key::F11),
        VK_F12 => Some(crate::Key::F12),
        VK_F13 => Some(crate::Key::F13),
        VK_F14 => Some(crate::Key::F14),
        VK_F15 => Some(crate::Key::F15),
        VK_F16 => Some(crate::Key::F16),
        VK_F17 => Some(crate::Key::F17),
        VK_F18 => Some(crate::Key::F18),
        VK_F19 => Some(crate::Key::F19),
        VK_F20 => Some(crate::Key::F20),
        VK_F21 => Some(crate::Key::F21),
        VK_F22 => Some(crate::Key::F22),
        VK_F23 => Some(crate::Key::F23),
        VK_F24 => Some(crate::Key::F24),
        VK_NUMLOCK => Some(crate::Key::NumLock),
        VK_SCROLL => Some(crate::Key::ScrollLock),
        VK_LSHIFT => Some(crate::Key::LeftShift),
        VK_RSHIFT => Some(crate::Key::RightShift),
        VK_SHIFT => {
            if event.MakeCode == 0x36 {
                Some(crate::Key::RightShift)
            } else {
                Some(crate::Key::LeftShift)
            }
        }
        VK_LCONTROL => Some(crate::Key::LeftControl),
        VK_RCONTROL => Some(crate::Key::RightControl),
        VK_CONTROL => {
            if e0 {
                Some(crate::Key::RightControl)
            } else {
                Some(crate::Key::LeftControl)
            }
        }
        VK_LMENU => Some(crate::Key::LeftAlt),
        VK_RMENU => Some(crate::Key::RightAlt),
        VK_MENU => {
            if e0 {
                Some(crate::Key::RightAlt)
            } else {
                Some(crate::Key::LeftAlt)
            }
        }
        VK_LWIN => Some(crate::Key::LeftMeta),
        VK_RWIN => Some(crate::Key::RightMeta),
        VK_APPS => Some(crate::Key::Menu),
        VK_SNAPSHOT => Some(crate::Key::PrintScreen),
        VK_PAUSE => Some(crate::Key::Pause),
        VK_CAPITAL => Some(crate::Key::CapsLock),
        VK_VOLUME_UP => Some(crate::Key::VolumeUp),
        VK_VOLUME_DOWN => Some(crate::Key::VolumeDown),
        VK_VOLUME_MUTE => Some(crate::Key::VolumeMute),
        VK_MEDIA_NEXT_TRACK => Some(crate::Key::MediaNext),
        VK_MEDIA_PREV_TRACK => Some(crate::Key::MediaPrevious),
        VK_MEDIA_STOP => Some(crate::Key::MediaStop),
        VK_MEDIA_PLAY_PAUSE => Some(crate::Key::MediaPlayPause),
        VK_NUMPAD0 => Some(crate::Key::Numpad0),
        VK_NUMPAD1 => Some(crate::Key::Numpad1),
        VK_NUMPAD2 => Some(crate::Key::Numpad2),
        VK_NUMPAD3 => Some(crate::Key::Numpad3),
        VK_NUMPAD4 => Some(crate::Key::Numpad4),
        VK_NUMPAD5 => Some(crate::Key::Numpad5),
        VK_NUMPAD6 => Some(crate::Key::Numpad6),
        VK_NUMPAD7 => Some(crate::Key::Numpad7),
        VK_NUMPAD8 => Some(crate::Key::Numpad8),
        VK_NUMPAD9 => Some(crate::Key::Numpad9),
        VK_MULTIPLY => Some(crate::Key::NumpadMultiply),
        VK_ADD => Some(crate::Key::NumpadAdd),
        VK_SUBTRACT => Some(crate::Key::NumpadSubtract),
        VK_DECIMAL => Some(crate::Key::NumpadDecimal),
        VK_DIVIDE => Some(crate::Key::NumpadDivide),
        _ => None,
    }
}

/// Creates a [`KeyCode`] instance from the provided [`RAWKEYBOARD`] event.
fn make_keycode(keyboard: &RAWKEYBOARD) -> KeyCode {
    let mut extended_flags = 0;
    if (keyboard.Flags as u32 & RI_KEY_E0) != 0 {
        extended_flags |= 0x01;
    }
    if (keyboard.Flags as u32 & RI_KEY_E1) != 0 {
        extended_flags |= 0x02;
    }

    KeyCode {
        code: keyboard.MakeCode,
        extended_flags,
    }
}

/// Handles a raw mouse event.
fn handle_mouse_event(device: HANDLE, mouse: &RAWMOUSE, state: &mut State) {
    use windows_sys::Win32::Devices::HumanInterfaceDevice::*;

    if (mouse.usFlags as u32 & MOUSE_MOVE_ABSOLUTE) == MOUSE_MOVE_RELATIVE
        && (mouse.lLastX != 0 || mouse.lLastY != 0)
    {
        state.send_event(crate::Event::MouseMoved {
            device: crate::Device(device),
            dx: mouse.lLastX as f64,
            dy: mouse.lLastY as f64,
        });
    }

    let btnflags = unsafe { mouse.Anonymous.Anonymous.usButtonFlags } as u32;

    if (btnflags & RI_MOUSE_LEFT_BUTTON_DOWN) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton::LEFT,
            pressed: true,
        });
    }

    if (btnflags & RI_MOUSE_LEFT_BUTTON_UP) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton::LEFT,
            pressed: false,
        });
    }

    if (btnflags & RI_MOUSE_MIDDLE_BUTTON_DOWN) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton::MIDDLE,
            pressed: true,
        });
    }

    if (btnflags & RI_MOUSE_MIDDLE_BUTTON_UP) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton::MIDDLE,
            pressed: false,
        });
    }

    if (btnflags & RI_MOUSE_RIGHT_BUTTON_DOWN) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton::RIGHT,
            pressed: true,
        });
    }

    if (btnflags & RI_MOUSE_RIGHT_BUTTON_UP) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton::RIGHT,
            pressed: false,
        });
    }

    if (btnflags & RI_MOUSE_BUTTON_4_DOWN) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton(4),
            pressed: true,
        });
    }

    if (btnflags & RI_MOUSE_BUTTON_4_UP) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton(4),
            pressed: false,
        });
    }

    if (btnflags & RI_MOUSE_BUTTON_5_DOWN) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton(5),
            pressed: true,
        });
    }

    if (btnflags & RI_MOUSE_BUTTON_5_UP) != 0 {
        state.send_event(crate::Event::MouseButton {
            device: crate::Device(device),
            button: crate::MouseButton(5),
            pressed: false,
        });
    }

    if (btnflags & RI_MOUSE_WHEEL) != 0 {
        let delta = unsafe { mouse.Anonymous.Anonymous.usButtonData } as i16 as f64;

        state.send_event(crate::Event::MouseWheel {
            device: crate::Device(device),
            dy: delta / WHEEL_DELTA as f64,
            dx: 0.0,
        });
    }

    if (btnflags & RI_MOUSE_HWHEEL) != 0 {
        let delta = unsafe { mouse.Anonymous.Anonymous.usButtonData } as i16 as f64;

        state.send_event(crate::Event::MouseWheel {
            device: crate::Device(device),
            dy: 0.0,
            dx: delta / WHEEL_DELTA as f64,
        });
    }
}

/// Returns whether the provided code is a low surrogate.
fn is_low_surrogate(code: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&code)
}

/// Returns whether the provided code is a high surrogate.
fn is_high_surrogate(code: u16) -> bool {
    (0xD800..=0xDBFF).contains(&code)
}

/// Decodes the provided low and high surrogates into an UTF-32 code point.
fn decode_utf16(low: u16, high: u16) -> Option<char> {
    std::char::decode_utf16([low, high])
        .next()
        .and_then(Result::ok)
}
