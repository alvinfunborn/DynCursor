use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub custom_layouts: String,
    pub cursor_mapping: String,
    pub enable_logging: bool,
    pub enabled: bool,
    pub poll_interval_ms: u64,
    pub stability_ms: u64,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub arrow_file: String,
    pub text_file: String,
    pub link_file: String,
}

impl Settings {
    pub fn from_values(values: &BTreeMap<String, String>) -> Result<Self, String> {
        let text =
            |key: &str, default: &str| values.get(key).cloned().unwrap_or_else(|| default.into());
        let boolean = |key: &str, default| match values.get(key).map(String::as_str) {
            None => Ok(default),
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            Some(_) => Err(format!("{} 必须是 true 或 false", key)),
        };
        let number = |key: &str, default, min, max| -> Result<u64, String> {
            let value = values
                .get(key)
                .map(|v| v.parse::<u64>())
                .transpose()
                .map_err(|_| format!("{} 必须是整数", key))?
                .unwrap_or(default);
            if !(min..=max).contains(&value) {
                return Err(format!("{} 必须在 {} 到 {} 之间", key, min, max));
            }
            Ok(value)
        };
        Ok(Self {
            custom_layouts: text("custom-layout", "0804"),
            cursor_mapping: text("cursor-mapping", "32512:32651, 32513:32516, 32649:32648"),
            enable_logging: boolean("enable-logging", false)?,
            enabled: boolean("enabled", true)?,
            poll_interval_ms: number("poll-interval-ms", 100, 50, 2000)?,
            stability_ms: number("stability-ms", 100, 0, 2000)?,
            start_minimized: boolean("start-minimized", false)?,
            close_to_tray: boolean("close-to-tray", true)?,
            arrow_file: text("arrow-file", ""),
            text_file: text("text-file", ""),
            link_file: text("link-file", ""),
        })
    }

    pub fn values(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("custom-layout".into(), self.custom_layouts.clone()),
            ("cursor-mapping".into(), self.cursor_mapping.clone()),
            ("enable-logging".into(), self.enable_logging.to_string()),
            ("enabled".into(), self.enabled.to_string()),
            ("poll-interval-ms".into(), self.poll_interval_ms.to_string()),
            ("stability-ms".into(), self.stability_ms.to_string()),
            ("start-minimized".into(), self.start_minimized.to_string()),
            ("close-to-tray".into(), self.close_to_tray.to_string()),
            ("arrow-file".into(), self.arrow_file.clone()),
            ("text-file".into(), self.text_file.clone()),
            ("link-file".into(), self.link_file.clone()),
        ])
    }

    pub fn cursor_files(&self, directory: &Path) -> Vec<(u32, PathBuf)> {
        [
            (32512, &self.arrow_file),
            (32513, &self.text_file),
            (32649, &self.link_file),
        ]
        .into_iter()
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(target, value)| {
            let path = PathBuf::from(value);
            (
                target,
                if path.is_absolute() {
                    path
                } else {
                    directory.join(path)
                },
            )
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_files_receive_safe_gui_defaults() {
        let settings = Settings::from_values(&BTreeMap::new()).unwrap();
        assert!(settings.close_to_tray);
        assert!(!settings.start_minimized);
        assert!(settings.enabled);
        assert_eq!(settings.poll_interval_ms, 100);
        assert!(settings.cursor_files(Path::new("C:/Apps")).is_empty());
        assert_eq!(Settings::from_values(&settings.values()).unwrap(), settings);
    }

    #[test]
    fn timing_and_boolean_inputs_are_bounded() {
        for (key, value) in [
            ("poll-interval-ms", "0"),
            ("poll-interval-ms", "2001"),
            ("stability-ms", "-1"),
            ("stability-ms", "2001"),
            ("close-to-tray", "yes"),
        ] {
            assert!(Settings::from_values(&BTreeMap::from([(key.into(), value.into())])).is_err());
        }
    }

    #[test]
    fn relative_cursor_files_use_program_directory() {
        let mut settings = Settings::from_values(&BTreeMap::new()).unwrap();
        settings.arrow_file = "resources/cur/custom-arrow.cur".into();
        assert_eq!(
            settings.cursor_files(Path::new("C:/Apps/DynCursor"))[0].1,
            Path::new("C:/Apps/DynCursor/resources/cur/custom-arrow.cur")
        );
    }
}
