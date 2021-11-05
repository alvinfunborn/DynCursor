// #![windows_subsystem = "windows"]
#[macro_use]
extern crate lazy_static;
extern crate log_panics;
extern crate z_lib;
use log4rs;
use log::info;
use std::thread::sleep;
mod yaml_client;
mod ime;
mod cursor;

const CUSTOM_LAYOUT: &str = "custom-layout";

fn listen_keyboard_layout() {
    let mut former_custom = false;
    let mut inited = false;
    let custom_layouts = yaml_client::get_config(CUSTOM_LAYOUT).unwrap();
    loop {
        sleep(std::time::Duration::from_millis(100));
        let (current_custom, locale_code, conversion) = ime::is_custom_ime(custom_layouts.clone());
        if !inited {
            if current_custom {
                info!("locale:{}, conversion:{}, init as custom ime", locale_code, conversion);
                cursor::change_cursor();
                former_custom = true;
            }
            inited = true;
        } else if current_custom != former_custom {
            info!("locale:{}, conversion:{}, change from former:{} to current:{}", locale_code, conversion, if former_custom { "custom ime" } else { "default ime" }, if current_custom { "custom ime" } else { "default ime" });
            cursor::change_cursor();
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