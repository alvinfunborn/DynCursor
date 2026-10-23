#![windows_subsystem = "windows"]

use std::path::PathBuf;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};

mod cursor;
mod engine;
mod gui;
mod ime;
mod instance;
mod settings;
mod yaml_client;

fn run(arguments: &[std::ffi::OsString]) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let config_path = yaml_client::config_path_for_executable(&executable)?;
    let mut background = false;
    let mut safe_mode = false;
    let mut smoke_test = false;
    match arguments.first().and_then(|value| value.to_str()) {
        Some("--version") if arguments.len() == 1 => {
            println!("DynCursor {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("--help") if arguments.len() == 1 => {
            println!("DynCursor [--version | --check-config [FILE] | --stop | --reset-cursors | --show | --background]");
            return Ok(());
        }
        Some("--check-config") if arguments.len() <= 2 => {
            let path = arguments.get(1).map(PathBuf::from).unwrap_or(config_path);
            let config = yaml_client::Config::load(&path)?;
            config.validate_files(executable.parent().ok_or("程序目录无效")?)?;
            println!("配置有效：{}", path.display());
            return Ok(());
        }
        Some("--stop") if arguments.len() == 1 => return instance::request_stop(),
        Some("--reset-cursors") if arguments.len() == 1 => {
            let _instance = instance::Instance::acquire()?;
            if unsafe {
                SystemParametersInfoW(
                    SPI_SETCURSORS,
                    0,
                    std::ptr::null_mut(),
                    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                )
            }
            .as_bool()
            {
                return Ok(());
            }
            return Err(format!("重置光标失败：{}", std::io::Error::last_os_error()));
        }
        Some("--show") if arguments.len() == 1 => {
            if instance::request_show()? {
                return Ok(());
            }
        }
        Some("--background") if arguments.len() == 1 => background = true,
        Some("--safe-mode") if arguments.len() == 1 => safe_mode = true,
        Some("--smoke-test") if arguments.len() == 1 => smoke_test = true,
        None if arguments.is_empty() => {}
        _ => return Err("未知参数，请使用 --help 查看用法".into()),
    }
    gui::run(config_path, background, safe_mode, smoke_test)
}

fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if let Err(error) = run(&arguments) {
        eprintln!("DynCursor: {}", error);
        if arguments.is_empty() {
            unsafe {
                MessageBoxW(
                    HWND::default(),
                    error.as_str(),
                    "DynCursor",
                    MB_OK | MB_ICONERROR,
                );
            }
        }
        std::process::exit(1);
    }
}
