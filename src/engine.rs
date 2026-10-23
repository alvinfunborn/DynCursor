use crate::{
    cursor::{CursorManager, WindowsBackend},
    ime,
    instance::Instance,
    yaml_client::Config,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub paused: bool,
    pub enabled: bool,
    pub custom: Option<bool>,
    pub locale: String,
    pub conversion: usize,
    pub error: Option<String>,
    pub version: String,
    pub config_path: String,
    pub smoke_test: bool,
}

type Reply = mpsc::Sender<Result<(), String>>;
enum Command {
    Ready(Reply),
    Apply(Box<Config>, Reply),
    Pause(bool, Reply),
    Recapture(bool, Reply),
    Stop(Reply),
}

pub struct Engine {
    commands: mpsc::Sender<Command>,
    status: Arc<Mutex<Status>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Default)]
struct Stabilizer {
    candidate: Option<(bool, u16, Instant)>,
}

impl Stabilizer {
    fn observe(&mut self, custom: bool, locale: u16, now: Instant, delay_ms: u64) -> bool {
        match self.candidate {
            Some((previous, previous_locale, since))
                if previous == custom && previous_locale == locale =>
            {
                now.saturating_duration_since(since) >= Duration::from_millis(delay_ms)
            }
            _ => {
                self.candidate = Some((custom, locale, now));
                delay_ms == 0
            }
        }
    }
}

fn capture(config: &Config, directory: &Path) -> Result<CursorManager<WindowsBackend>, String> {
    config.validate_files(directory)?;
    let mut replacements = BTreeMap::new();
    for (target, path) in config.settings.cursor_files(directory) {
        replacements.insert(
            target,
            WindowsBackend::load_file(&path)
                .map_err(|error| format!("无法加载光标 {}：{}", path.display(), error))?,
        );
    }
    CursorManager::new(WindowsBackend, config.cursor_mapping.clone())
        .map(|manager| manager.with_replacements(replacements))
        .map_err(|error| format!("无法保存系统光标：{}", error))
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        config: Config,
        config_path: PathBuf,
        directory: PathBuf,
        instance: Instance,
        initial_error: Option<String>,
        safe_mode: bool,
        smoke_test: bool,
        on_show: impl Fn() + Send + 'static,
        on_exit: impl Fn() + Send + 'static,
    ) -> Self {
        let (commands, receiver) = mpsc::channel();
        let status = Arc::new(Mutex::new(Status {
            paused: safe_mode || smoke_test || initial_error.is_some(),
            enabled: config.settings.enabled,
            custom: None,
            locale: "等待输入状态".into(),
            conversion: 0,
            error: initial_error,
            version: env!("CARGO_PKG_VERSION").into(),
            config_path: config_path.display().to_string(),
            smoke_test,
        }));
        let runtime = status.clone();
        let thread = thread::spawn(move || {
            let mut config = config;
            let mut manager = match capture(&config, &directory) {
                Ok(manager) => Some(manager),
                Err(error) => {
                    let mut state = runtime.lock().unwrap();
                    state.error = Some(error);
                    state.paused = true;
                    None
                }
            };
            let mut applied = Some(false);
            let mut stabilizer = Stabilizer::default();
            let mut retry_after = Instant::now();
            loop {
                if instance.stopping(0).unwrap_or(true) {
                    break;
                }
                if instance.show_requested() {
                    on_show();
                }
                let timeout = Duration::from_millis(config.settings.poll_interval_ms);
                match receiver.recv_timeout(timeout) {
                    Ok(Command::Ready(reply)) => {
                        let state = runtime.lock().unwrap();
                        let result = match (&manager, &state.error) {
                            (Some(_), None) => Ok(()),
                            (_, Some(error)) => Err(error.clone()),
                            _ => Err("系统光标尚未就绪".into()),
                        };
                        let _ = reply.send(result);
                    }
                    Ok(Command::Stop(reply)) => {
                        let result = manager
                            .as_mut()
                            .map_or(Ok(()), |m| m.restore().map_err(|e| e.to_string()));
                        let _ = reply.send(result);
                        break;
                    }
                    Ok(Command::Pause(paused, reply)) => {
                        // Pause stays active even if restoring a cursor fails.
                        runtime.lock().unwrap().paused = paused;
                        let result = if paused {
                            manager
                                .as_mut()
                                .map_or(Ok(()), |m| m.restore().map_err(|e| e.to_string()))
                        } else if manager.is_none() {
                            Err("尚未成功保存系统光标，请修正设置后重新捕获".into())
                        } else {
                            Ok(())
                        };
                        if result.is_err() {
                            runtime.lock().unwrap().paused = true;
                        }
                        applied = if result.is_ok() && paused {
                            Some(false)
                        } else {
                            None
                        };
                        stabilizer = Stabilizer::default();
                        runtime.lock().unwrap().error = result.as_ref().err().cloned();
                        let _ = reply.send(result);
                    }
                    Ok(Command::Apply(candidate, reply)) => {
                        let result = (|| {
                            if let Some(current) = manager.as_mut() {
                                current.restore().map_err(|e| e.to_string())?;
                            }
                            let replacement = capture(&candidate, &directory)?;
                            candidate.save(&config_path)?;
                            manager = Some(replacement);
                            config = *candidate;
                            Ok(())
                        })();
                        applied = if result.is_ok() { Some(false) } else { None };
                        stabilizer = Stabilizer::default();
                        let mut state = runtime.lock().unwrap();
                        state.enabled = config.settings.enabled;
                        state.error = result.as_ref().err().cloned();
                        let _ = reply.send(result);
                    }
                    Ok(Command::Recapture(resume, reply)) => {
                        let result = (|| {
                            if let Some(current) = manager.as_mut() {
                                current.restore().map_err(|e| e.to_string())?;
                            }
                            manager = Some(capture(&config, &directory)?);
                            Ok(())
                        })();
                        applied = if result.is_ok() { Some(false) } else { None };
                        stabilizer = Stabilizer::default();
                        let mut state = runtime.lock().unwrap();
                        state.paused = !resume || result.is_err();
                        state.error = result.as_ref().err().cloned();
                        let _ = reply.send(result);
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let paused = runtime.lock().unwrap().paused || !config.settings.enabled;
                runtime.lock().unwrap().custom = applied;
                if paused {
                    if applied != Some(false) {
                        let result = manager
                            .as_mut()
                            .map_or(Ok(()), |m| m.restore().map_err(|e| e.to_string()));
                        if result.is_ok() {
                            applied = Some(false);
                        }
                    }
                    continue;
                }
                let state = match ime::current_state(&config.custom_layouts) {
                    Some(state) => state,
                    None => {
                        runtime.lock().unwrap().locale = "当前窗口状态不可读".into();
                        stabilizer = Stabilizer::default();
                        continue;
                    }
                };
                {
                    let mut status = runtime.lock().unwrap();
                    status.locale = format!("{:04X}", state.locale);
                    status.conversion = state.conversion;
                }
                if applied == Some(state.custom) || Instant::now() < retry_after {
                    if applied == Some(state.custom) {
                        stabilizer = Stabilizer::default();
                    }
                    continue;
                }
                if !stabilizer.observe(
                    state.custom,
                    state.locale,
                    Instant::now(),
                    config.settings.stability_ms,
                ) {
                    continue;
                }
                let result = manager
                    .as_mut()
                    .ok_or_else(|| "系统光标尚未就绪".to_string())
                    .and_then(|m| m.set_custom(state.custom).map_err(|e| e.to_string()));
                match result {
                    Ok(()) => {
                        applied = Some(state.custom);
                        runtime.lock().unwrap().custom = applied;
                        runtime.lock().unwrap().error = None;
                        if config.enable_logging {
                            log::info!(
                                "locale:{:04x}, conversion:{}, custom:{}",
                                state.locale,
                                state.conversion,
                                state.custom
                            );
                        }
                    }
                    Err(error) => {
                        applied = None;
                        runtime.lock().unwrap().custom = None;
                        runtime.lock().unwrap().error = Some(error);
                        retry_after = Instant::now() + Duration::from_millis(500);
                    }
                }
            }
            // Restore before notifying Tauri that shutdown may finish.
            drop(manager);
            on_exit();
        });
        Self {
            commands,
            status,
            thread: Mutex::new(Some(thread)),
        }
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    fn request(&self, make: impl FnOnce(Reply) -> Command) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(make(reply))
            .map_err(|_| "后台检测线程已停止".to_string())?;
        response
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "后台操作未在 5 秒内完成".to_string())?
    }

    pub fn apply(&self, config: Config) -> Result<(), String> {
        self.request(|reply| Command::Apply(Box::new(config), reply))
    }
    pub fn ready(&self) -> Result<(), String> {
        self.request(Command::Ready)
    }
    pub fn pause(&self, paused: bool) -> Result<(), String> {
        self.request(|reply| Command::Pause(paused, reply))
    }
    pub fn recapture(&self, resume: bool) -> Result<(), String> {
        self.request(|reply| Command::Recapture(resume, reply))
    }

    pub fn shutdown(&self) {
        let mut thread = self.thread.lock().unwrap();
        if let Some(handle) = thread.take() {
            let _ = self.request(Command::Stop);
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_and_state_must_stay_stable_for_configured_duration() {
        let start = Instant::now();
        let mut state = Stabilizer::default();
        assert!(!state.observe(true, 0x0804, start, 100));
        assert!(!state.observe(true, 0x0804, start + Duration::from_millis(99), 100));
        assert!(state.observe(true, 0x0804, start + Duration::from_millis(100), 100));
        assert!(!state.observe(false, 0x0804, start + Duration::from_millis(101), 100));
        assert!(!state.observe(false, 0x0409, start + Duration::from_millis(202), 100));
        assert!(state.observe(false, 0x0409, start + Duration::from_millis(302), 100));
        assert!(Stabilizer::default().observe(true, 0x0804, start, 0));
    }
}
