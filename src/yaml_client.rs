extern crate z_lib;
use z_lib::z_serde_yaml::Yaml;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::io::Error;

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