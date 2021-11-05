use log::debug;
use std::ptr;
use windows::{
    Win32::UI::Input::Ime::*,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
};

/// 获取当前是否是自定义语言
fn get_keyboard_layout() -> String {
    unsafe {
        let hkl = GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(), ptr::null_mut()));
        let hkl = format!("{:x}", hkl.0);
        debug!("current locale:{}", hkl);

        let len = hkl.len();
        let locale_code = &hkl[len - 4..];
        locale_code.to_string()
    }
}

/// 当前语言输入法的conversionMode是否是默认(即不convert即英文)
fn is_default_conversion() -> (bool, isize) {
    unsafe {
        let hime = ImmGetDefaultIMEWnd(GetForegroundWindow());
        let status = SendMessageA(hime, WM_IME_CONTROL, WPARAM(1), LPARAM(0));
        debug!("conversion mode status:{:?}", status);
        (status.0 == 0, status.0)
    }
}

/// 当前语言是否需要检查conversionMode
fn need_check_conversion(locale_code: &str) -> bool {
    // locale_code.eq("0804")
    !locale_code.eq("0409")
}

pub fn is_custom_ime(custom_layouts: String) -> (bool, String, isize) {
    let locale_code = get_keyboard_layout();
    let is_custom_layout = custom_layouts.contains(locale_code.as_str());
    let mut conversion = 0;
    let mut is_custom_ime = false;
    if is_custom_layout && need_check_conversion(locale_code.as_str()) {
        let (is_default_conversion, current_conversion) = is_default_conversion();
        is_custom_ime = !is_default_conversion;
        conversion = current_conversion;
    } else {
        is_custom_ime = is_custom_layout;
    }
    (is_custom_ime, locale_code, conversion)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        loop {
            println!("is custom ime: {}", super::is_custom_ime("0804".to_string()).0);
            std::thread::sleep(std::time::Duration::from_millis(3000));
        }
    }
}