use crate::components::char::CharAttributes;
use crate::components::char::Percentage;
use crate::components::skills::skill::SkillCastingAttributes;
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
pub struct DevConfigStatsPlayerJob {
    #[serde(flatten)]
    pub attributes: CharAttributes,
    pub mounted_speedup: Percentage,
}

#[derive(Debug, Deserialize)]
pub struct DevConfigStatsPlayer {
    pub crusader: DevConfigStatsPlayerJob,
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
    pub skills: SkillsConfig,
}

#[derive(Debug, Deserialize)]
pub struct SkillConfigFireWall {
    pub pushback_force: f32,
    pub damage: u32,
    pub width: u16,
    pub duration_seconds: f32,
    pub force_duration_seconds: f32,
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
}

#[derive(Debug, Deserialize)]
pub struct SkillConfigPyroBlast {
    pub moving_speed: f32,
    pub damage: u32,
    pub ball_size: f32,
    pub splash_radius: f32,
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
}

#[derive(Debug, Deserialize)]
pub struct SkillConfigHeal {
    pub heal: u32,
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
}

#[derive(Debug, Deserialize)]
pub struct SkillConfigBrutalTestSkill {
    pub damage: u32,
    pub width: u16,
    pub height: u16,
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
}

#[derive(Debug, Deserialize)]
pub struct LightningSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
}

#[derive(Debug, Deserialize)]
pub struct PoisonSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub damage: u32,
}

#[derive(Debug, Deserialize)]
pub struct FireBombSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub damage: u32,
}

#[derive(Debug, Deserialize)]
pub struct AbsorbShieldSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub duration_seconds: f32,
}

#[derive(Debug, Deserialize)]
pub struct SkillsConfig {
    pub firewall: SkillConfigFireWall,
    pub wiz_pyroblast: SkillConfigPyroBlast,
    pub heal: SkillConfigHeal,
    pub brutal_test_skill: SkillConfigBrutalTestSkill,
    pub lightning: LightningSkillConfig,
    pub mounting: SkillCastingAttributes,
    pub unmounting: SkillCastingAttributes,
    pub cure: SkillCastingAttributes,
    pub poison: PoisonSkillConfig,
    pub firebomb: FireBombSkillConfig,
    pub absorb_shield: AbsorbShieldSkillConfig,
}

impl DevConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let mut s = config::Config::new();
        s.merge(config::File::with_name("config-runtime"))?;
        return s.try_into();
    }
}
