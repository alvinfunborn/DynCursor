use log::debug;
use std::collections::BTreeSet;
use std::ptr;
use windows::{
    Win32::Foundation::*, Win32::Globalization::IME_CMODE_NATIVE, Win32::UI::Input::Ime::*,
    Win32::UI::Input::KeyboardAndMouse::*, Win32::UI::WindowsAndMessaging::*,
};

const IMC_GETCONVERSIONMODE: usize = 1;
const IMC_GETOPENSTATUS: usize = 5;
const IME_TIMEOUT_MS: u32 = 100;

#[derive(Debug)]
pub struct ImeState {
    pub custom: bool,
    pub locale: u16,
    pub conversion: usize,
}

fn locale_code(layout: isize) -> u16 {
    (layout as usize & 0xffff) as u16
}

fn needs_conversion(locale: u16, is_ime: bool) -> bool {
    // Modern TSF IMEs may not be reported by ImmIsIME. Include CJK primary languages.
    is_ime || matches!(locale & 0x03ff, 0x04 | 0x11 | 0x12)
}

fn native_conversion(mode: usize) -> bool {
    mode & IME_CMODE_NATIVE as usize != 0
}

fn query_ime(window: HWND, command: usize) -> Option<usize> {
    let mut result = 0;
    let success = unsafe {
        SendMessageTimeoutW(
            window,
            WM_IME_CONTROL,
            WPARAM(command),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
            IME_TIMEOUT_MS,
            &mut result,
        )
    };
    (success.0 != 0).then_some(result)
}

/// An unavailable window or timed-out IME is unknown, rather than an English state.
pub fn current_state(custom_layouts: &BTreeSet<u16>) -> Option<ImeState> {
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.0 == 0 {
            return None;
        }
        let thread = GetWindowThreadProcessId(foreground, ptr::null_mut());
        if thread == 0 {
            return None;
        }
        let layout = GetKeyboardLayout(thread);
        if layout.0 == 0 {
            return None;
        }
        let locale = locale_code(layout.0);
        let mut custom = custom_layouts.contains(&locale);
        let mut conversion = 0;
        if custom && needs_conversion(locale, ImmIsIME(layout).as_bool()) {
            let ime_window = ImmGetDefaultIMEWnd(foreground);
            if ime_window.0 == 0 {
                return None;
            }
            if query_ime(ime_window, IMC_GETOPENSTATUS)? != 0 {
                conversion = query_ime(ime_window, IMC_GETCONVERSIONMODE)?;
            }
            custom = native_conversion(conversion);
        }
        if GetForegroundWindow() != foreground {
            return None;
        }
        debug!(
            "locale:{:04x}, conversion:{}, custom:{}",
            locale, conversion, custom
        );
        Some(ImeState {
            custom,
            locale,
            conversion,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_id_is_extracted_without_string_slicing() {
        assert_eq!(locale_code(0), 0);
        assert_eq!(locale_code(0x0804), 0x0804);
        assert_eq!(locale_code(0x0409_0409), 0x0409);
        assert_eq!(locale_code(-0x7ff8_f7fc), 0x0804);
    }

    #[test]
    fn alphabetic_mode_flags_do_not_mean_native_input() {
        assert!(!native_conversion(0));
        assert!(!native_conversion(0x08)); // Full-width alphanumeric.
        assert!(!native_conversion(0x10)); // Roman flag alone.
        assert!(native_conversion(0x01));
        assert!(native_conversion(0x09));
    }

    #[test]
    fn ordinary_keyboards_do_not_require_an_ime() {
        assert!(!needs_conversion(0x0409, false));
        assert!(!needs_conversion(0x0809, false));
        assert!(!needs_conversion(0x040c, false));
        assert!(needs_conversion(0x0804, false));
        assert!(needs_conversion(0x0411, false));
        assert!(needs_conversion(0x0412, false));
        assert!(needs_conversion(0x0409, true));
    }
}
