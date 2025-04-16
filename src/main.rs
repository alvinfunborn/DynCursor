#![windows_subsystem = "windows"]
#[macro_use]
extern crate lazy_static;
extern crate log_panics;
extern crate z_lib;
use log4rs;
use log::info;
use std::thread::sleep;
use std::time::Duration;
mod yaml_client;
mod ime;
mod cursor;

const CUSTOM_LAYOUT: &str = "custom-layout";
const ENABLE_LOGGING: &str = "enable-logging";

fn listen_keyboard_layout() {
    let mut former_custom = false;
    let mut inited = false;
    let custom_layouts = yaml_client::get_config(CUSTOM_LAYOUT).unwrap();
    
    // 初始化时等待系统稳定
    sleep(Duration::from_millis(500));
    
    loop {
        // 主循环间隔
        sleep(Duration::from_millis(100));
        
        // 获取当前状态
        let (current_custom, locale_code, conversion) = ime::is_custom_ime(custom_layouts.clone());
        
        // 如果状态发生变化，等待一段时间让系统稳定
        if !inited || current_custom != former_custom {
            sleep(Duration::from_millis(100));
            let (verify_custom, _, _) = ime::is_custom_ime(custom_layouts.clone());
            
            // 如果状态不稳定，跳过本次循环
            if verify_custom != current_custom {
                continue;
            }
            
            // 执行光标更改
            if !inited {
                if current_custom {
                    info!("locale:{}, conversion:{}, init as custom ime", locale_code, conversion);
                    cursor::change_cursor();
                    former_custom = true;
                }
                inited = true;
            } else if current_custom != former_custom {
                info!("locale:{}, conversion:{}, change from former:{} to current:{}", 
                    locale_code, conversion,
                    if former_custom { "custom ime" } else { "default ime" },
                    if current_custom { "custom ime" } else { "default ime" }
                );
                cursor::change_cursor();
                former_custom = current_custom;
            }
            
            // 状态变化后额外等待一段时间
            sleep(Duration::from_millis(50));
        }
    }
}

pub fn init() {
    if let Some(enable_logging) = yaml_client::get_config(ENABLE_LOGGING) {
        if enable_logging == "true" {
            log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
            log_panics::init();
            info!("log inited...");
        }
    }
}

fn main() {
    init();
    listen_keyboard_layout();
}