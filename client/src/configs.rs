use crate::components::controller::CastMode;
use rustarok_common::char_attr::CharAttributes;
use rustarok_common::common::Percentage;
use rustarok_common::config::{CommonConfigs, DevConfigStats, SkillsConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub username: String,
    pub max_fps: usize,
    pub log_level: String,
    pub resolution_w: u32,
    pub resolution_h: u32,
    pub grf_paths: Vec<String>,
    pub server_addr: String,
    pub load_models: bool,
    pub load_sprites: bool,
    pub cast_mode: CastMode,
    pub lerping_ticks: usize,
    pub lerping_enabled: bool,
    pub show_last_acknowledged_pos: bool,
}

impl AppConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let mut s = config::Config::new();
        s.merge(config::File::with_name("config"))?;
        return s.try_into();
    }
}
