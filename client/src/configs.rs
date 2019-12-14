use crate::components::char::CharAttributes;
use crate::components::char::Percentage;
use crate::components::controller::CastMode;
use crate::components::skills::skills::SkillCastingAttributes;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub log_level: String,
    pub start_pos_x: f32,
    pub start_pos_y: f32,
    pub resolution_w: u32,
    pub resolution_h: u32,
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
    pub gunslinger: DevConfigStatsPlayerJob,
    pub hunter: DevConfigStatsPlayerJob,
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
    pub execute_script: String,
    pub sleep_ms: u64,
    pub minions_enabled: bool,
    pub stats: DevConfigStats,
    pub console: DevConfigConsole,
    pub network: DevConfigNetwork,
    pub skills: SkillsConfig,
    pub cast_mode: CastMode,
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

#[derive(Debug, Deserialize, Clone)]
pub struct SkillConfigPyroBlastInner {
    pub moving_speed: f32,
    pub damage: u32,
    pub secondary_damage: u32,
    pub ball_size: f32,
    pub splash_radius: f32,
}

#[derive(Debug, Deserialize)]
pub struct SkillConfigPyroBlast {
    #[serde(flatten)]
    pub inner: SkillConfigPyroBlastInner,
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
    pub width: f32,
    pub height: f32,
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
}

#[derive(Debug, Deserialize)]
pub struct SkillConfigSanctuarySkill {
    pub heal: u32,
    pub heal_freq_seconds: f32,
    pub duration: f32,
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
    pub duration_seconds: f32,
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
pub struct ExoSkeletonSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub duration_seconds: f32,
    pub armor: Percentage,
    pub attack_damage: Percentage,
    pub attack_range: Percentage,
    pub movement_speed: Percentage,
    pub attack_speed: Percentage,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AssaBladeDashSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub duration_seconds: f32,
    pub first_damage: u32,
    pub second_damage: u32,
}

#[derive(Debug, Deserialize)]
pub struct AssaPhasePrismSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub duration_seconds: f32,
    pub swap_duration_unit_per_second: f32,
    pub damage: u32,
}

#[derive(Debug, Deserialize)]
pub struct FalconCarry {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub carry_ally_duration: f32,
    pub carry_owner_duration: f32,
}

#[derive(Debug, Deserialize)]
pub struct FalconAttack {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub damage: u32,
    pub slow: Percentage,
    pub duration_in_seconds: f32,
    pub slow_duration: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GazXplodiumChargeSkillConfigInner {
    pub missile_travel_duration_seconds: f32,
    pub detonation_duration: f32,
    pub damage: u32,
    pub stun_duration_seconds: f32,
    pub explosion_area: f32,
}

#[derive(Debug, Deserialize)]
pub struct GazXplodiumChargeSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    #[serde(flatten)]
    pub inner: GazXplodiumChargeSkillConfigInner,
}

#[derive(Debug, Deserialize)]
pub struct GazTurretSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub turret: CharAttributes,
}

#[derive(Debug, Deserialize)]
pub struct GazBarricadeSkillConfig {
    #[serde(flatten)]
    pub attributes: SkillCastingAttributes,
    pub max_hp: i32,
    pub armor: Percentage,
    pub hp_regen: Percentage,
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
    pub exoskeleton: ExoSkeletonSkillConfig,
    pub assa_blade_dash: AssaBladeDashSkillConfig,
    pub assa_phase_prism: AssaPhasePrismSkillConfig,
    pub falcon_carry: FalconCarry,
    pub falcon_attack: FalconAttack,
    pub gaz_xplodium_charge: GazXplodiumChargeSkillConfig,
    pub gaz_turret: GazTurretSkillConfig,
    pub gaz_barricade: GazBarricadeSkillConfig,
    pub gaz_destroy_turret: SkillCastingAttributes,
    pub sanctuary: SkillConfigSanctuarySkill,
}

impl DevConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let mut s = config::Config::new();
        s.merge(config::File::with_name("config-runtime"))?;
        return s.try_into();
    }
}
