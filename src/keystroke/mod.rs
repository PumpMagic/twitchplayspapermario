//! Send a string, character, or keystroke event to the system.

pub use self::platform::{press_key, release_key};
pub use self::platform::{send_key, send_combo};
pub use self::platform::{send_char, send_str};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Physical {
    Return,
    Control,
    Alt,
    Shift,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Scan {
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Key {
    Physical(Physical),
    Unicode(char),
    Scan(Scan)
}

#[cfg(target_os = "macos")]
mod platform {

}

#[cfg(target_os = "windows")]
mod platform {
    extern crate winapi;
    extern crate user32 as user32_sys;

    use std::mem::{size_of, transmute_copy};
    use self::winapi::{c_int, WORD};
    use self::winapi::{INPUT_KEYBOARD, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, KEYEVENTF_SCANCODE};
    use self::winapi::{INPUT, LPINPUT, KEYBDINPUT, MOUSEINPUT};
    use self::winapi::{VK_RETURN, VK_SHIFT, VK_CONTROL, VK_MENU, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12};
    use self::user32_sys::SendInput;

    use super::{Physical, Key, Scan};

    fn get_keycode(p: Physical) -> WORD {
        use super::Physical::*;
        match p {
            Return => VK_RETURN as WORD,
            Shift => VK_SHIFT as WORD,
            Control => VK_CONTROL as WORD,
            Alt => VK_MENU as WORD,
            F1 => VK_F1 as WORD,
            F2 => VK_F2 as WORD,
            F3 => VK_F3 as WORD,
            F4 => VK_F4 as WORD,
            F5 => VK_F5 as WORD,
            F6 => VK_F6 as WORD,
            F7 => VK_F7 as WORD,
            F8 => VK_F8 as WORD,
            F9 => VK_F9 as WORD,
            F10 => VK_F10 as WORD,
            F11 => VK_F11 as WORD,
            F12 => VK_F12 as WORD,
            A => 'A' as WORD,
            B => 'B' as WORD,
            C => 'C' as WORD,
            D => 'D' as WORD,
            E => 'E' as WORD,
            F => 'F' as WORD,
            G => 'G' as WORD,
            H => 'H' as WORD,
            I => 'I' as WORD,
            J => 'J' as WORD,
            K => 'K' as WORD,
            L => 'L' as WORD,
            M => 'M' as WORD,
            N => 'N' as WORD,
            O => 'O' as WORD,
            P => 'P' as WORD,
            Q => 'Q' as WORD,
            R => 'R' as WORD,
            S => 'S' as WORD,
            T => 'T' as WORD,
            U => 'U' as WORD,
            V => 'V' as WORD,
            W => 'W' as WORD,
            X => 'X' as WORD,
            Y => 'Y' as WORD,
            Z => 'Z' as WORD,
        }
    }
        
    pub fn get_scancode(s: Scan) -> WORD {
        use super::Scan::*;
        match s {
            F1 => 0x3B as WORD,
            F2 => 0x3C as WORD,
            F3 => 0x3D as WORD,
            F4 => 0x3E as WORD,
            F5 => 0x3F as WORD,
            F6 => 0x40 as WORD,
            F7 => 0x41 as WORD,
            F8 => 0x42 as WORD,
            F9 => 0x43 as WORD,
            F10 => 0x44 as WORD,
            F11 => 0x85 as WORD,
            F12 => 0x86 as WORD
        }
    }

    pub fn press_key(k: Key) {
        unsafe { match k {
            Key::Physical(p) => {
                let mut x = INPUT {
                    type_: INPUT_KEYBOARD,
                    u: [0u64; 4],
                };
                unsafe {
                    *x.ki_mut() = KEYBDINPUT {
                        wVk: get_keycode(p), // 'a' key
                        wScan: 0, // 0 := hardware scan code for a key
                        dwFlags: 0, // 0 := a key press
                        time: 0,
                        dwExtraInfo: 0,
                    };
                }
                
                SendInput(1, &mut x as LPINPUT, size_of::<INPUT>() as c_int);
            },
            Key::Unicode(c) => {
                let mut x = INPUT {
                    type_: INPUT_KEYBOARD,
                    u: [0u64; 4],
                };
                unsafe {
                    *x.ki_mut() = KEYBDINPUT {
                        wVk: 0,
                        wScan: c as WORD, // 0 := hardware scan code for a key
                        dwFlags: KEYEVENTF_UNICODE, // 0 := a key press
                        time: 0,
                        dwExtraInfo: 0,
                    };
                }
                
                SendInput(1, &mut x as LPINPUT, size_of::<INPUT>() as c_int);
            },
            Key::Scan(sc) => {
                let mut x = INPUT {
                    type_: INPUT_KEYBOARD,
                    u: [0u64; 4],
                };
                unsafe {
                    *x.ki_mut() = KEYBDINPUT {
                        wVk: 0,
                        wScan: get_scancode(sc),
                        dwFlags: KEYEVENTF_SCANCODE,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                }
                
                SendInput(1, &mut x as LPINPUT, size_of::<INPUT>() as c_int);
            }
        }}
    }

    pub fn release_key(k: Key) {
        unsafe { match k {
            Key::Physical(p) => {
                let mut x = INPUT {
                    type_: INPUT_KEYBOARD,
                    u: [0u64; 4],
                };
                unsafe {
                    *x.ki_mut() = KEYBDINPUT {
                        wVk: get_keycode(p), // 'a' key
                        wScan: 0, // 0 := hardware scan code for a key
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                }
                
                SendInput(1, &mut x as LPINPUT, size_of::<INPUT>() as c_int);
            },
            Key::Unicode(c) => {
                let mut x = INPUT {
                    type_: INPUT_KEYBOARD,
                    u: [0u64; 4],
                };
                unsafe {
                    *x.ki_mut() = KEYBDINPUT {
                        wVk: 0, // 'a' key
                        wScan: c as WORD, // 0 := hardware scan code for a key
                        dwFlags: KEYEVENTF_UNICODE|KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                }
                
                SendInput(1, &mut x as LPINPUT, size_of::<INPUT>() as c_int);
            },
            Key::Scan(sc) => {
                let mut x = INPUT {
                    type_: INPUT_KEYBOARD,
                    u: [0u64; 4],
                };
                unsafe {
                    *x.ki_mut() = KEYBDINPUT {
                        wVk: 0, // 'a' key
                        wScan: get_scancode(sc),
                        dwFlags: KEYEVENTF_SCANCODE|KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                }
                
                SendInput(1, &mut x as LPINPUT, size_of::<INPUT>() as c_int);
            }
        }}
    }

    pub fn send_combo(keys: &[Key]) {
        for &k in keys.iter() {
            press_key(k);
        }
        for &k in keys.iter().rev() {
            release_key(k);
        }
    }

    pub fn send_key(k: Key) {
        press_key(k);
        release_key(k);
    }

    /// Send all unicode characters below 0x10000, silently skipping others.
    pub fn send_char(c: char) {
        if (c as u64) < 0x10000 {
            send_key(Key::Unicode(c));
        }
    }

    /// Send a string as keyboard events
    pub fn send_str(msg: &str) {
        for c in msg.chars() {
            send_char(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::send_str;

    #[test]
    fn test_lowercase_str() {
        send_str("echo");
    }
}
