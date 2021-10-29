#![windows_subsystem = "windows"]
#[macro_use]
extern crate lazy_static;
extern crate log_panics;
extern crate z_lib;
extern crate winapi;
use log4rs;
use log::{error, info, warn, debug};
use z_lib::z_serde_yaml::Yaml;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::thread::sleep;
use std::{ptr};
use std::io::Error;
use winapi::um::winuser::*;

lazy_static! {
    static ref DEFAULT_YAML: Mutex<Yaml> = init_yaml();
}

fn init_yaml() -> Mutex<Yaml> {
    Mutex::new(Yaml::new("config/config.yaml"))
}

pub fn get_config(key: &str) -> Option<String> {
    let yaml = DEFAULT_YAML.lock().unwrap();
    yaml.get(key)
}

pub fn set_config(key: &str, v: String) -> Result<(), Error> {
    let mut yaml = DEFAULT_YAML.lock().unwrap();
    yaml.set(key, v)
}

pub fn get_map_config(key: &str) -> Option<BTreeMap<String, String>> {
    let map_str = get_config(key);
    match map_str {
        Some(map_str) => {
            let mut map = BTreeMap::new();
            let split = map_str.split(",");
            for s in split {
                let kv = s.trim().split(":").collect::<Vec<&str>>();
                map.insert(kv[0].trim().to_string(), kv[1].trim().to_string());
            }
            Some(map)
        },
        None => None,
    }
}

const CUSTOM_LAYOUT: &str = "custom-layout";
const CURSOR_MAPPING: &str = "cursor-mapping";

/// 获取当前是否是自定义输入法
fn is_custom_keyboard_layout() -> bool {
    unsafe {
        let hkl = GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(), ptr::null_mut()));
        let hkl = format!("{:?}", hkl);
        debug!("current locale:{:?}", hkl);

        let len = hkl.len();
        let locale_code = &hkl[len - 4..];
        let custom_layouts = get_config(CUSTOM_LAYOUT).unwrap();
        custom_layouts.contains(locale_code)
    }
}

fn change_cursor() {
    unsafe {
        let map = get_map_config(CURSOR_MAPPING).unwrap_or_default();
        for (k, v) in map.iter() {
            let cursor = LoadCursorW(ptr::null_mut(), MAKEINTRESOURCEW(v.parse::<u16>().unwrap_or_default()));
            info!("replace cursor:{} with cursor:{}. error code:{:?}", k, v, SetSystemCursor(cursor, k.parse::<u32>().unwrap_or_default()));
        }
        // // 标准箭头+问号 替换 标准箭头
        // let idc: u16 = 32651; // HELP(ARROW WITH QUESTION MARK)
        // let cursor = LoadCursorW(ptr::null_mut(), MAKEINTRESOURCEW(idc));
        // info!("replace ARROW(32512) with {}:{:?}", idc, SetSystemCursor(cursor, 32512));
        // // 向上箭头 替换 输入IBEAM
        // let idc: u16 = 32516; // UPARROW
        // let cursor = LoadCursorW(ptr::null_mut(), MAKEINTRESOURCEW(idc));
        // info!("replace IBEAM(32513) with {}:{:?}", idc, SetSystemCursor(cursor, 32513));
        
        // 通过LoadImage加载cursor, cx和cy不起作用?
        // let cursor = LoadImageW(ptr::null_mut(), MAKEINTRESOURCEW(32516), IMAGE_CURSOR, 32, 32, LR_SHARED) as HCURSOR; 
    }
}

fn listen_keyboard_layout() {
    let mut former_custom = false;
    let mut inited = false;
    loop {
        sleep(std::time::Duration::from_millis(100));
        let current_custom = is_custom_keyboard_layout();
        if !inited {
            if current_custom {
                info!("init as custom layout");
                change_cursor();
                former_custom = true;
            }
            inited = true;
        } else if current_custom != former_custom {
            info!("change from former:{} to current:{}", if former_custom { "custom layout" } else { "default layout" }, if current_custom { "custom layout" } else { "default layout" });
            change_cursor();
            former_custom = current_custom;
        }
    }
}

pub fn init() {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    log_panics::init();
    info!("log inited...")
}

fn main() {
    init();
    listen_keyboard_layout();
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_map_config() {
        // println!("{:?}", get_map_config("cursor-mapping"));
    }
}