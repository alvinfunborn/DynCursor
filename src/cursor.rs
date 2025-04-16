use log::info;
use std::{thread::sleep, time::Duration};
use crate::yaml_client;
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
};

const CURSOR_MAPPING: &str = "cursor-mapping";
const MAX_RETRIES: u32 = 3;

/// 修改指针样式
pub fn change_cursor() {
    let map = yaml_client::get_map_config(CURSOR_MAPPING).unwrap_or_default();
    for (k, v) in map.iter() {
        let mut success = false;
        let mut retries = 0;
        while !success && retries < MAX_RETRIES {
            success = replace_cursor(v.parse::<i32>().unwrap_or_default(), k.parse::<u32>().unwrap_or_default());
            if !success {
                sleep(Duration::from_millis(50));
                retries += 1;
            }
        }
        // 确保系统有时间处理光标更改
        sleep(Duration::from_millis(10));
    }
}

fn replace_cursor(c1: i32, c2: u32) -> bool {
    unsafe {
        let cursor = LoadCursorW(HINSTANCE::default(), PWSTR(c1 as _));
        if cursor.0 == 0 {
            info!("Failed to load cursor:{}", c1);
            return false;
        }
        
        let result = SetSystemCursor(cursor, SYSTEM_CURSOR_ID(c2));
        let success = result.as_bool();
        
        if success {
            info!("Successfully replaced cursor:{} with cursor:{}", c1, c2);
        } else {
            info!("Failed to replace cursor:{} with cursor:{}. Error code: {:?}", c1, c2, result);
        }
        
        success
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