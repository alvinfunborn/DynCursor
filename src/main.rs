#![windows_subsystem = "windows"]

use log::{info, warn};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};

mod cursor;
mod ime;
mod instance;
mod yaml_client;

fn listen_keyboard_layout(
    config: yaml_client::Config,
    instance: &instance::Instance,
) -> Result<(), String> {
    let mut cursors = cursor::CursorManager::new(cursor::WindowsBackend, config.cursor_mapping)
        .map_err(|error| format!("无法保存原始光标：{}", error))?;
    let mut applied = Some(false);
    sleep(Duration::from_millis(500));
    while !instance.stopping(100)? {
        let state = match ime::current_state(&config.custom_layouts) {
            Some(state) => state,
            None => continue,
        };
        if applied == Some(state.custom) {
            continue;
        }
        if instance.stopping(100)? {
            break;
        }
        let stable = ime::current_state(&config.custom_layouts)
            .map(|verified| verified.custom == state.custom && verified.locale == state.locale)
            .unwrap_or(false);
        if !stable {
            continue;
        }
        match cursors.set_custom(state.custom) {
            Ok(()) => {
                applied = Some(state.custom);
                info!(
                    "locale:{:04x}, conversion:{}, custom:{}",
                    state.locale, state.conversion, state.custom
                );
            }
            Err(error) => {
                applied = None;
                warn!("切换光标失败，将重试：{}", error);
                if instance.stopping(500)? {
                    break;
                }
            }
        }
    }
    cursors
        .restore()
        .map_err(|error| format!("恢复原始光标失败：{}", error))
}

fn run(arguments: &[std::ffi::OsString]) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let config_path = yaml_client::config_path_for_executable(&executable)?;
    match arguments.first().and_then(|value| value.to_str()) {
        Some("--version") if arguments.len() == 1 => {
            println!("DynCursor {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("--help") if arguments.len() == 1 => {
            println!("DynCursor [--version | --check-config [FILE] | --stop | --reset-cursors]");
            return Ok(());
        }
        Some("--check-config") if arguments.len() <= 2 => {
            let path = arguments.get(1).map(PathBuf::from).unwrap_or(config_path);
            yaml_client::Config::load(&path)?;
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
        None if arguments.is_empty() => {}
        _ => return Err("未知参数，请使用 --help 查看用法".into()),
    }
    let instance = instance::Instance::acquire()?;
    let config = yaml_client::Config::load(&config_path)?;
    // Relative paths inside log4rs.yaml must also use the application directory.
    let directory = executable.parent().ok_or("无法确定程序目录")?;
    std::env::set_current_dir(directory).map_err(|error| error.to_string())?;
    if config.enable_logging {
        log4rs::init_file(directory.join("config/log4rs.yaml"), Default::default())
            .map_err(|error| format!("无法初始化日志：{}", error))?;
        log_panics::init();
        info!("DynCursor {} started", env!("CARGO_PKG_VERSION"));
    }
    listen_keyboard_layout(config, &instance)
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
