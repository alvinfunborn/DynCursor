use std::io;
use std::ptr;
use windows::{
    Win32::Foundation::*, Win32::System::SystemServices::EVENT_MODIFY_STATE,
    Win32::System::Threading::*,
};

const MUTEX_NAME: &str = "Local\\DynCursor.Instance";
const STOP_EVENT_NAME: &str = "Local\\DynCursor.Stop";

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub struct Instance {
    show: OwnedHandle,
    // Drop the stop event before releasing the instance lock.
    stop: OwnedHandle,
    _mutex: OwnedHandle,
}

impl Instance {
    pub fn acquire() -> Result<Self, String> {
        Self::acquire_named(MUTEX_NAME, STOP_EVENT_NAME)
    }

    pub fn acquire_for_launch(mode: crate::launch::LaunchMode) -> Result<Option<Self>, String> {
        Self::acquire_named_or_activate(
            MUTEX_NAME,
            STOP_EVENT_NAME,
            Some(mode.activates_existing()),
        )
    }

    pub fn acquire_smoke_test() -> Result<Self, String> {
        let name = format!("Local\\DynCursor.Smoke.{}", std::process::id());
        Self::acquire_named(&format!("{}.Mutex", name), &format!("{}.Stop", name))
    }

    fn acquire_named(mutex_name: &str, stop_event_name: &str) -> Result<Self, String> {
        Self::acquire_named_or_activate(mutex_name, stop_event_name, None)?
            .ok_or_else(|| "DynCursor 已在运行".into())
    }

    fn acquire_named_or_activate(
        mutex_name: &str,
        stop_event_name: &str,
        activate_existing: Option<bool>,
    ) -> Result<Option<Self>, String> {
        unsafe {
            let handle = CreateMutexW(ptr::null(), false, mutex_name);
            let last_error = GetLastError();
            if handle.0 == 0 {
                return Err(format!("无法创建实例锁：{}", io::Error::last_os_error()));
            }
            let mutex = OwnedHandle(handle);
            if last_error == ERROR_ALREADY_EXISTS {
                return match activate_existing {
                    Some(false) => Ok(None),
                    Some(true) => {
                        if request_show_named(&format!("{}.Show", stop_event_name))? {
                            Ok(None)
                        } else {
                            Err("DynCursor 正在启动，请稍后再打开设置".into())
                        }
                    }
                    None => Err("DynCursor 已在运行，请先使用 DynCursor.exe --stop 退出".into()),
                };
            }
            let handle = CreateEventW(ptr::null(), true, false, stop_event_name);
            if handle.0 == 0 {
                return Err(format!("无法创建退出事件：{}", io::Error::last_os_error()));
            }
            let stop = OwnedHandle(handle);
            let show = CreateEventW(
                ptr::null(),
                false,
                false,
                format!("{}.Show", stop_event_name).as_str(),
            );
            if show.0 == 0 {
                return Err(format!(
                    "无法创建窗口显示事件：{}",
                    io::Error::last_os_error()
                ));
            }
            Ok(Some(Self {
                show: OwnedHandle(show),
                stop,
                _mutex: mutex,
            }))
        }
    }

    pub fn stopping(&self, timeout_ms: u32) -> Result<bool, String> {
        let result = unsafe { WaitForSingleObject(self.stop.0, timeout_ms) };
        if result == WAIT_OBJECT_0 {
            Ok(true)
        } else if result == WAIT_TIMEOUT.0 {
            Ok(false)
        } else {
            Err(format!("等待退出事件失败：{}", io::Error::last_os_error()))
        }
    }

    pub fn show_requested(&self) -> bool {
        unsafe { WaitForSingleObject(self.show.0, 0) == WAIT_OBJECT_0 }
    }
}

fn request_show_named(show_event_name: &str) -> Result<bool, String> {
    unsafe {
        let handle = OpenEventW(EVENT_MODIFY_STATE, false, show_event_name);
        if handle.0 == 0 {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND.0 as i32) {
                Ok(false)
            } else {
                Err(error.to_string())
            };
        }
        let event = OwnedHandle(handle);
        if SetEvent(event.0).as_bool() {
            Ok(true)
        } else {
            Err(io::Error::last_os_error().to_string())
        }
    }
}

pub fn request_stop() -> Result<(), String> {
    request_stop_named(STOP_EVENT_NAME)
}

fn request_stop_named(stop_event_name: &str) -> Result<(), String> {
    unsafe {
        let handle = OpenEventW(EVENT_MODIFY_STATE, false, stop_event_name);
        if handle.0 == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND.0 as i32) {
                return Ok(());
            }
            return Err(format!("无法请求退出：{}", error));
        }
        let event = OwnedHandle(handle);
        if SetEvent(event.0).as_bool() {
            Ok(())
        } else {
            Err(format!("无法请求退出：{}", io::Error::last_os_error()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_background_launch_does_not_signal_the_settings_window() {
        use crate::launch::LaunchMode;
        let name = format!("Local\\DynCursor.BackgroundTest.{}", std::process::id());
        let mutex_name = format!("{}.Mutex", name);
        let stop_name = format!("{}.Stop", name);
        let instance = Instance::acquire_named(&mutex_name, &stop_name).unwrap();
        for mode in [LaunchMode::Background, LaunchMode::Normal, LaunchMode::Show] {
            assert!(Instance::acquire_named_or_activate(
                &mutex_name,
                &stop_name,
                Some(mode.activates_existing()),
            )
            .unwrap()
            .is_none());
            assert_eq!(instance.show_requested(), mode.activates_existing());
            assert!(!instance.stopping(0).unwrap());
        }
    }

    #[test]
    fn instance_lock_and_stop_event_have_a_clean_lifecycle() {
        // Private names ensure this test cannot signal a user's running application.
        let mutex_name = format!("Local\\DynCursor.Test.{}.Mutex", std::process::id());
        let stop_name = format!("Local\\DynCursor.Test.{}.Stop", std::process::id());
        request_stop_named(&stop_name).unwrap();
        let instance = Instance::acquire_named(&mutex_name, &stop_name).unwrap();
        assert!(Instance::acquire_named(&mutex_name, &stop_name).is_err());
        assert!(!instance.stopping(0).unwrap());
        assert!(!instance.show_requested());
        assert!(unsafe { SetEvent(instance.show.0) }.as_bool());
        assert!(instance.show_requested());
        assert!(!instance.show_requested());
        request_stop_named(&stop_name).unwrap();
        assert!(instance.stopping(0).unwrap());
        drop(instance);
        let restarted = Instance::acquire_named(&mutex_name, &stop_name).unwrap();
        assert!(!restarted.stopping(0).unwrap());
    }
}
