use log::info;
use crate::yaml_client;
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
};

const CURSOR_MAPPING: &str = "cursor-mapping";

/// 修改指针样式
pub fn change_cursor() {
    let map = yaml_client::get_map_config(CURSOR_MAPPING).unwrap_or_default();
    for (k, v) in map.iter() {
        replace_cursor(v.parse::<i32>().unwrap_or_default(), k.parse::<u32>().unwrap_or_default());
    }
}

fn replace_cursor(c1: i32, c2: u32) {
    unsafe {
        let cursor = LoadCursorW(HINSTANCE::default(), PWSTR(c1 as _));
        info!("replace cursor:{} with cursor:{}. error code:{:?}", c1, c2, SetSystemCursor(cursor, SYSTEM_CURSOR_ID(c2)));
    }
}

// fn IDC_MAP(c: u16) -> 

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        unsafe {
            let cursor = LoadCursorW(HINSTANCE::default(), IDC_ARROW);
            println!("error code:{:?}", SetSystemCursor(cursor, OCR_HELP));
        }
}
}