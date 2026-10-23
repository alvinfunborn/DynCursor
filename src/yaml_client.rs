use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CURSOR_IDS: &[u32] = &[
    32512, 32513, 32514, 32515, 32516, 32642, 32643, 32644, 32645, 32646, 32648, 32649, 32650,
    32651,
];

#[derive(Clone, Debug)]
pub struct Config {
    pub custom_layouts: BTreeSet<u16>,
    pub cursor_mapping: BTreeMap<u32, u32>,
    pub enable_logging: bool,
    pub settings: crate::settings::Settings,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|error| format!("无法读取配置 {}：{}", path.display(), error))?;
        Self::parse(&content).map_err(|error| format!("配置 {} 无效：{}", path.display(), error))
    }

    fn parse(content: &str) -> Result<Self, String> {
        // Preserve the original scalar YAML format, including unquoted 0804/false.
        let values: BTreeMap<String, String> =
            serde_yaml::from_str(content).map_err(|error| error.to_string())?;
        let layouts = values.get("custom-layout").ok_or("缺少 custom-layout")?;
        let mut custom_layouts = BTreeSet::new();
        for layout in layouts.split(',').map(str::trim) {
            if layout.len() != 4 || !layout.bytes().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!("输入法编码必须是四位十六进制数字：{}", layout));
            }
            let code = u16::from_str_radix(layout, 16).map_err(|error| error.to_string())?;
            if code == 0 {
                return Err("输入法编码不能为 0000".into());
            }
            custom_layouts.insert(code);
        }

        let mappings = values.get("cursor-mapping").ok_or("缺少 cursor-mapping")?;
        let mut cursor_mapping = BTreeMap::new();
        for mapping in mappings.split(',').map(str::trim) {
            let (target, source) = mapping
                .split_once(':')
                .ok_or_else(|| format!("光标映射必须使用 key:value 格式：{}", mapping))?;
            let parse_id = |value: &str| {
                value
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|id| CURSOR_IDS.contains(id))
                    .ok_or_else(|| format!("无效或不支持的光标编码：{}", value.trim()))
            };
            let target = parse_id(target)?;
            let source = parse_id(source)?;
            if cursor_mapping.insert(target, source).is_some() {
                return Err(format!("光标映射的目标重复：{}", target));
            }
        }

        let enable_logging = match values.get("enable-logging").map(String::as_str) {
            None | Some("false") => false,
            Some("true") => true,
            Some(other) => return Err(format!("enable-logging 必须是 true 或 false：{}", other)),
        };
        Ok(Self {
            custom_layouts,
            cursor_mapping,
            enable_logging,
            settings: crate::settings::Settings::from_values(&values)?,
        })
    }

    pub fn from_settings(settings: crate::settings::Settings) -> Result<Self, String> {
        let text = serde_yaml::to_string(&settings.values()).map_err(|e| e.to_string())?;
        Self::parse(&text)
    }

    pub fn defaults() -> Self {
        Self::parse(include_str!("../config/config.yaml")).expect("bundled configuration is valid")
    }

    pub fn validate_files(&self, directory: &Path) -> Result<(), String> {
        for (_, path) in self.settings.cursor_files(directory) {
            if !path
                .extension()
                .map(|s| s.eq_ignore_ascii_case("cur"))
                .unwrap_or(false)
            {
                return Err(format!("请选择 .cur 静态光标文件：{}", path.display()));
            }
            let metadata = fs::metadata(&path)
                .map_err(|e| format!("无法读取光标 {}：{}", path.display(), e))?;
            if !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 {
                return Err(format!("光标必须是 2 MB 以内的文件：{}", path.display()));
            }
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let mut values: BTreeMap<String, String> = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_yaml::from_str(&text).ok())
            .unwrap_or_default();
        values.extend(self.settings.values());
        let text = serde_yaml::to_string(&values).map_err(|e| e.to_string())?;
        let parent = path.parent().ok_or("配置目录无效")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temporary = parent.join(format!(
            ".config-{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        ));
        fs::write(&temporary, text).map_err(|e| e.to_string())?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("无法保存配置 {}：{}", path.display(), error));
        }
        Ok(())
    }
}

pub fn config_path_for_executable(executable: &Path) -> Result<PathBuf, String> {
    executable
        .parent()
        .map(|directory| directory.join("config/config.yaml"))
        .ok_or_else(|| "无法确定程序目录".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_config_is_backward_compatible() {
        let config = Config::parse(include_str!("../config/config.yaml")).unwrap();
        assert_eq!(config.custom_layouts, BTreeSet::from([0x0804]));
        assert_eq!(config.cursor_mapping.get(&32512), Some(&32651));
        assert!(!config.enable_logging);
    }

    #[test]
    fn saving_preserves_unknown_keys_and_failed_save_preserves_original() {
        let directory = std::env::temp_dir().join(format!(
            "DynCursor-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("config.yaml");
        fs::write(
            &path,
            "custom-layout: 0804\ncursor-mapping: 32512:32651\nuser-option: keep-me\n",
        )
        .unwrap();
        let mut config = Config::load(&path).unwrap();
        config.settings.start_minimized = true;
        config.save(&path).unwrap();
        assert!(Config::load(&path).unwrap().settings.start_minimized);
        let values: BTreeMap<String, String> =
            serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(values["user-option"], "keep-me");
        let original = fs::read(&path).unwrap();
        let lock = fs::OpenOptions::new().read(true).open(&path).unwrap();
        // A directory in the destination position must fail without leaving temporary files.
        let bad_target = directory.join("existing-directory");
        fs::create_dir(&bad_target).unwrap();
        assert!(config.save(&bad_target).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
        drop(lock);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&bad_target).unwrap();
        fs::remove_dir(&directory).unwrap();
    }

    #[test]
    fn bundled_logging_config_is_compatible_with_updated_log4rs() {
        // Deserialize without opening the configured files or installing a global logger.
        let _: log4rs::config::RawConfig =
            serde_yaml::from_str(include_str!("../config/log4rs.yaml")).unwrap();
    }

    #[test]
    fn multiple_layouts_are_parsed_as_complete_codes() {
        let config = Config::parse(
            "custom-layout: 0804, 0809, 0C04\ncursor-mapping: 32512:32651\nenable-logging: true",
        )
        .unwrap();
        assert!(config.custom_layouts.contains(&0x0809));
        assert!(config.custom_layouts.contains(&0x0c04));
        assert!(!config.custom_layouts.contains(&0x0409));
        assert!(config.enable_logging);
    }

    #[test]
    fn malformed_mappings_return_errors_instead_of_panicking() {
        for mapping in [
            "32512",
            "",
            "32512:",
            ":32651",
            "32512:32651,",
            "x:32651",
            "32512:99999",
            "32512:32651:32513",
            "32512:32651,32512:32516",
        ] {
            let text = format!("custom-layout: 0804\ncursor-mapping: '{}'", mapping);
            assert!(Config::parse(&text).is_err(), "accepted {}", mapping);
        }
    }

    #[test]
    fn invalid_layouts_and_logging_are_rejected() {
        for layout in ["", "804", "0000", "zzzz", "0804,", "00804"] {
            let text = format!("custom-layout: '{}'\ncursor-mapping: 32512:32651", layout);
            assert!(Config::parse(&text).is_err());
        }
        assert!(Config::parse("cursor-mapping: 32512:32651").is_err());
        assert!(Config::parse("custom-layout: 0804").is_err());
        assert!(Config::parse(
            "custom-layout: 0804\ncursor-mapping: 32512:32651\nenable-logging: yes"
        )
        .is_err());
    }

    #[test]
    fn configuration_is_resolved_relative_to_executable() {
        let executable = Path::new("C:/Apps/DynCursor/DynCursor.exe");
        assert_eq!(
            config_path_for_executable(executable).unwrap(),
            Path::new("C:/Apps/DynCursor/config/config.yaml")
        );
        let error =
            Config::load(Path::new("__missing_dyncursor_config__/config.yaml")).unwrap_err();
        assert!(error.contains("__missing_dyncursor_config__"));
    }
}
