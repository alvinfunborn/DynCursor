#[macro_use]
extern crate lazy_static;

use std::fs;
use std::collections::BTreeMap;
use std::io::Error;
use std::sync::Mutex;

pub struct Yaml {
    path: String,
    map: BTreeMap<String, String>,
}

impl Yaml {
    pub fn new(path: &str) -> Yaml {
        let content = fs::read_to_string(path).expect("error reading file");
        let map = serde_yaml::from_str(&content).unwrap();
        Yaml { path: String::from(path), map }
    }

    pub fn set(&mut self, key: &str, v: String) -> Result<(), Error> {
        &self.map.insert(key.to_string(), v);
        let s = serde_yaml::to_string(&self.map).unwrap();
        fs::write(&self.path, s)
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let v = self.map.get(key);
        match v {
            Some(d) => Some(d.to_string()),
            None => None,
        }
    }
}

lazy_static! {
    static ref DEFAULT_YAML: Mutex<Yaml> = init_yaml();
}

fn init_yaml() -> Mutex<Yaml> {
    Mutex::new(Yaml::new("config.yaml"))
}

pub fn get(key: &str) -> Option<String> {
    let yaml = DEFAULT_YAML.lock().unwrap();
    yaml.get(key)
}

pub fn set(key: &str, v: String) -> Result<(), Error> {
    let mut yaml = DEFAULT_YAML.lock().unwrap();
    yaml.set(key, v)
}

#[cfg(test)]
mod test {
    #[test]
    fn test1() {

    }
}
