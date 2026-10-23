use crate::{
    engine::{Engine, Status},
    instance::Instance,
    settings::Settings,
    yaml_client::Config,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

static LOGGING_READY: AtomicBool = AtomicBool::new(false);

struct AppState {
    engine: Arc<Engine>,
    config: Mutex<Config>,
    config_path: PathBuf,
    directory: PathBuf,
    pause_item: MenuItem<tauri::Wry>,
    smoke_test: bool,
}

#[derive(Serialize)]
struct SettingsView {
    settings: Settings,
    autostart: bool,
    config_path: String,
    directory: String,
}

fn configure_logging(enabled: bool, directory: &Path) -> Result<(), String> {
    if enabled && !LOGGING_READY.load(Ordering::Relaxed) {
        log4rs::init_file(directory.join("config/log4rs.yaml"), Default::default())
            .map_err(|e| format!("无法启用日志：{}", e))?;
        log_panics::init();
        LOGGING_READY.store(true, Ordering::Relaxed);
    }
    log::set_max_level(if enabled {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Off
    });
    Ok(())
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn sync_tray(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        let status = state.engine.status();
        let paused = status.paused || !status.enabled;
        let _ = state.pause_item.set_text(if paused {
            "恢复动态切换"
        } else {
            "暂停并恢复原始光标"
        });
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(if paused {
                "DynCursor · 已暂停"
            } else {
                "DynCursor · 正在运行"
            }));
        }
    }
}

#[tauri::command]
fn get_status(state: tauri::State<'_, AppState>) -> Status {
    state.engine.status()
}

#[tauri::command]
fn get_settings(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<SettingsView, String> {
    Ok(SettingsView {
        settings: state.config.lock().unwrap().settings.clone(),
        autostart: app.autolaunch().is_enabled().map_err(|e| e.to_string())?,
        config_path: state.config_path.display().to_string(),
        directory: state.directory.display().to_string(),
    })
}

#[tauri::command]
async fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut current = state.config.lock().unwrap();
        let candidate = Config::from_settings(settings)?;
        candidate.validate_files(&state.directory)?;
        configure_logging(candidate.enable_logging, &state.directory)?;
        if let Err(error) = state.engine.apply(candidate.clone()) {
            let _ = configure_logging(current.enable_logging, &state.directory);
            return Err(error);
        }
        *current = candidate;
        let settings = current.settings.clone();
        drop(current);
        sync_tray(&app);
        Ok(settings)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn default_settings() -> Settings {
    Config::defaults().settings
}

#[tauri::command]
async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if app.state::<AppState>().smoke_test {
            return Err("测试模式不会修改登录自启动".into());
        }
        let manager = app.autolaunch();
        if enabled {
            manager.enable()
        } else {
            manager.disable()
        }
        .map_err(|e| e.to_string())?;
        manager.is_enabled().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pick_cursor_file(app: AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("选择静态光标文件")
            .add_filter("Windows 光标", &["cur"])
            .blocking_pick_file()
            .map(|file| {
                file.into_path()
                    .map(|p| p.display().to_string())
                    .map_err(|e| e.to_string())
            })
            .transpose()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn cursor_preview(path: String, app: AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut settings = state.config.lock().unwrap().settings.clone();
        settings.arrow_file = path;
        settings.text_file.clear();
        settings.link_file.clear();
        let config = Config::from_settings(settings)?;
        config.validate_files(&state.directory)?;
        let path = config
            .settings
            .cursor_files(&state.directory)
            .into_iter()
            .next()
            .ok_or("未选择文件")?
            .1;
        preview_bytes(std::fs::read(path).map_err(|e| e.to_string())?)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn preview_bytes(mut bytes: Vec<u8>) -> Result<String, String> {
    use base64::Engine as _;
    if bytes.len() < 22 || bytes[0..4] != [0, 0, 2, 0] {
        return Err("不是有效的静态 CUR 文件".into());
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if count == 0 || count > 64 || bytes.len() < 6 + count * 16 {
        return Err("CUR 图像目录无效".into());
    }
    bytes[2] = 1;
    for i in 0..count {
        let entry = 6 + i * 16;
        bytes[entry + 4..entry + 6].copy_from_slice(&1u16.to_le_bytes());
        bytes[entry + 6..entry + 8].copy_from_slice(&32u16.to_le_bytes());
    }
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Ico);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(512);
    limits.max_image_height = Some(512);
    limits.max_alloc = Some(16 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| e.to_string())?
        .thumbnail(64, 64);
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png.into_inner())
    ))
}

fn launch(program: &str, argument: &std::ffi::OsStr) -> Result<(), String> {
    std::process::Command::new(program)
        .arg(argument)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn perform_action(app: AppHandle, action: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        match action.as_str() {
            "show" => show_window(&app),
            "pause" => {
                let status = state.engine.status();
                if !status.enabled {
                    return Err("自动切换已在设置中关闭，请先启用并保存".into());
                }
                state.engine.pause(!status.paused)?;
            }
            "restore" => state.engine.pause(true)?,
            "recapture" => state.engine.recapture(true)?,
            "mouse" => {
                state.engine.pause(true)?;
                launch("control.exe", std::ffi::OsStr::new("main.cpl,,1"))?;
            }
            "reset" => {
                state.engine.pause(true)?;
                use windows::Win32::UI::WindowsAndMessaging::*;
                if !unsafe {
                    SystemParametersInfoW(
                        SPI_SETCURSORS,
                        0,
                        std::ptr::null_mut(),
                        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                    )
                }
                .as_bool()
                {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                state.engine.recapture(false)?;
            }
            "cursors" => launch(
                "explorer.exe",
                state.directory.join("resources/cur").as_os_str(),
            )?,
            "config" => launch("explorer.exe", state.directory.join("config").as_os_str())?,
            "logs" => {
                let path = state.directory.join("log");
                std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
                launch("explorer.exe", path.as_os_str())?;
            }
            "guide" => launch(
                "explorer.exe",
                std::ffi::OsStr::new("https://alvinfunborn.github.io/DynCursor/guide.html"),
            )?,
            "release" => launch(
                "explorer.exe",
                std::ffi::OsStr::new("https://github.com/alvinfunborn/DynCursor/releases/latest"),
            )?,
            "license" => launch("notepad.exe", state.directory.join("LICENSE").as_os_str())?,
            "reload" => {
                let mut current = state.config.lock().unwrap();
                let candidate = Config::load(&state.config_path)?;
                configure_logging(candidate.enable_logging, &state.directory)?;
                if let Err(error) = state.engine.apply(candidate.clone()) {
                    let _ = configure_logging(current.enable_logging, &state.directory);
                    return Err(error);
                }
                *current = candidate;
            }
            "quit" => {
                state.engine.shutdown();
                app.exit(0);
            }
            _ => return Err("不支持的操作".into()),
        }
        sync_tray(&app);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn action(app: AppHandle, action: String) -> Result<(), String> {
    perform_action(app, action).await
}

#[tauri::command]
fn smoke_ready(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    if !state.smoke_test {
        return Err("仅供烟雾测试使用".into());
    }
    state.engine.ready()?;
    println!("DynCursor GUI ready: WebView2, settings IPC and cursor snapshots succeeded");
    state.engine.shutdown();
    app.exit(0);
    Ok(())
}

pub fn run(
    config_path: PathBuf,
    background: bool,
    safe_mode: bool,
    smoke_test: bool,
) -> Result<(), String> {
    let instance = match if smoke_test {
        Instance::acquire_smoke_test()
    } else {
        Instance::acquire()
    } {
        Ok(instance) => instance,
        Err(error) => {
            if crate::instance::request_show()? {
                return Ok(());
            }
            return Err(error);
        }
    };
    let directory = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("程序目录无效")?
        .to_path_buf();
    std::env::set_current_dir(&directory).map_err(|e| e.to_string())?;
    let (config, mut initial_error) = match Config::load(&config_path) {
        Ok(config) => (config, None),
        Err(error) => (
            Config::defaults(),
            Some(format!(
                "{}。当前显示默认值，请修正后保存并恢复运行。",
                error
            )),
        ),
    };
    if let Err(error) = configure_logging(config.enable_logging, &directory) {
        initial_error = Some(error);
    }
    let initial_hidden =
        smoke_test || (initial_error.is_none() && (background || config.settings.start_minimized));
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_settings,
            save_settings,
            default_settings,
            set_autostart,
            pick_cursor_file,
            cursor_preview,
            action,
            smoke_ready
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let settings_item = MenuItem::with_id(&handle, "show", "打开设置", true, None::<&str>)?;
            let pause_item =
                MenuItem::with_id(&handle, "pause", "暂停并恢复原始光标", true, None::<&str>)?;
            let mouse_item =
                MenuItem::with_id(&handle, "mouse", "Windows 指针设置…", true, None::<&str>)?;
            let capture_item = MenuItem::with_id(
                &handle,
                "recapture",
                "重新捕获并恢复运行",
                true,
                None::<&str>,
            )?;
            let exit_item =
                MenuItem::with_id(&handle, "quit", "恢复光标并退出", true, None::<&str>)?;
            let menu = Menu::with_items(
                &handle,
                &[
                    &settings_item,
                    &pause_item,
                    &mouse_item,
                    &capture_item,
                    &exit_item,
                ],
            )?;
            TrayIconBuilder::with_id("main")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../resources/logo.ico"
                ))?)
                .tooltip("DynCursor")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    let app = app.clone();
                    let action = event.id.as_ref().to_string();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = perform_action(app.clone(), action).await {
                            show_window(&app);
                            app.dialog().message(error).title("DynCursor").show(|_| {});
                        }
                    });
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } | TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        }
                    ) {
                        show_window(tray.app_handle());
                    }
                })
                .build(&handle)?;
            let on_show = handle.clone();
            let on_exit = handle.clone();
            let engine = Arc::new(Engine::start(
                config.clone(),
                config_path.clone(),
                directory.clone(),
                instance,
                initial_error,
                safe_mode,
                smoke_test,
                move || show_window(&on_show),
                move || on_exit.exit(0),
            ));
            app.manage(AppState {
                engine,
                config: Mutex::new(config),
                config_path,
                directory,
                pause_item,
                smoke_test,
            });
            if !initial_hidden {
                show_window(&handle);
            }
            sync_tray(&handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.config.lock().unwrap().settings.close_to_tray {
                    if window.hide().is_ok() {
                        api.prevent_close();
                    }
                } else {
                    state.engine.shutdown();
                }
            }
        })
        .build(tauri::generate_context!())
        .map_err(|e| format!("无法启动设置窗口：{}", e))?;
    app.run(|app, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            if let Some(state) = app.try_state::<AppState>() {
                state.engine.shutdown();
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cur_previews_decode_real_bundled_artwork_and_reject_invalid_headers() {
        for name in ["custom-arrow.cur", "custom-beam.cur", "custom-link.cur"] {
            let bytes = std::fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("resources/cur")
                    .join(name),
            )
            .unwrap();
            assert!(preview_bytes(bytes)
                .unwrap()
                .starts_with("data:image/png;base64,iVBOR"));
        }
        for bytes in [
            vec![],
            vec![0; 22],
            include_bytes!("../resources/logo.ico").to_vec(),
        ] {
            assert!(preview_bytes(bytes).is_err());
        }
    }
}
