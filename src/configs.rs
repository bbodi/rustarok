use crate::components::char::CharAttributes;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub log_level: String,
    pub quick_startup: bool,
    pub grf_paths: Vec<String>,
}

impl AppConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let mut s = config::Config::new();
        s.merge(config::File::with_name("config"))?;
        return s.try_into();
    }
}

#[derive(Debug, Deserialize)]
pub struct DevConfigStats {
    pub minion: DevConfigStatsMinion,
    pub player: DevConfigStatsPlayer,
}

#[derive(Debug, Deserialize)]
pub struct DevConfigStatsMinion {
    pub melee: CharAttributes,
    pub ranged: CharAttributes,
}

#[derive(Debug, Deserialize)]
pub struct DevConfigStatsPlayer {
    pub crusader: CharAttributes,
}

#[derive(Debug, Deserialize)]
pub struct DevConfigConsole {
    pub color: [u8; 4],
}

#[derive(Debug, Deserialize)]
pub struct DevConfigNetwork {
    pub send_render_data_every_nth_frame: u64,
}

#[derive(Debug, Deserialize)]
pub struct DevConfig {
    pub sleep_ms: u64,
    pub minions_enabled: bool,
    pub stats: DevConfigStats,
    pub console: DevConfigConsole,
    pub network: DevConfigNetwork,
}

impl DevConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let mut s = config::Config::new();
        s.merge(config::File::with_name("config-runtime"))?;
        return s.try_into();
    }
}
