use rustarok_common::config::{CommonConfigs, DevConfigStats, SkillsConfig};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {}

impl ServerConfig {
    pub fn new(filename: &str) -> Result<Self, config::ConfigError> {
        let mut s = config::Config::new();
        s.merge(config::File::with_name(filename))?;
        return s.try_into();
    }
}

pub fn load_common_configs(filename: &str) -> Result<CommonConfigs, config::ConfigError> {
    let mut s = config::Config::new();
    s.merge(config::File::with_name(filename))?;
    return s.try_into();
}
