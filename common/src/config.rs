use crate::char_attr::CharAttributes;
use crate::common::{LocalTime, Percentage};
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommonConfigs {
    pub stats: DevConfigStats,
    pub skills: SkillsConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DevConfigStats {
    pub minion: DevConfigStatsMinion,
    pub player: DevConfigStatsPlayer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DevConfigStatsMinion {
    pub melee: CharAttributes,
    pub ranged: CharAttributes,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DevConfigStatsPlayerJob {
    pub attributes: CharAttributes,
    pub mounted_speedup: Percentage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DevConfigStatsPlayer {
    pub crusader: DevConfigStatsPlayerJob,
    pub gunslinger: DevConfigStatsPlayerJob,
    pub hunter: DevConfigStatsPlayerJob,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SkillCastingAttributes {
    pub casting_time: LocalTime,
    pub cast_delay: LocalTime,
    pub casting_range: f32,
    // in case of Directional skills
    pub width: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SkillConfigFireWall {
    pub pushback_force: f32,
    pub damage: u32,
    pub width: u16,
    pub duration_seconds: f32,
    pub force_duration_seconds: f32,
    pub attributes: SkillCastingAttributes,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillConfigPyroBlastInner {
    pub moving_speed: f32,
    pub damage: u32,
    pub secondary_damage: u32,
    pub ball_size: f32,
    pub splash_radius: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SkillConfigPyroBlast {
    pub inner: SkillConfigPyroBlastInner,
    pub attributes: SkillCastingAttributes,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SkillConfigHeal {
    pub heal: u32,
    pub attributes: SkillCastingAttributes,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SkillConfigBrutalTestSkill {
    pub damage: u32,
    pub width: f32,
    pub height: f32,
    pub attributes: SkillCastingAttributes,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SkillConfigSanctuarySkill {
    pub heal: u32,
    pub heal_freq_seconds: f32,
    pub duration: f32,
    pub attributes: SkillCastingAttributes,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LightningSkillConfig {
    pub attributes: SkillCastingAttributes,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PoisonSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub damage: u32,
    pub duration_seconds: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FireBombSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub damage: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AbsorbShieldSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub duration_seconds: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExoSkeletonSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub duration_seconds: f32,
    pub armor: Percentage,
    pub attack_damage: Percentage,
    pub attack_range: Percentage,
    pub movement_speed: Percentage,
    pub attack_speed: Percentage,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AssaBladeDashSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub duration_seconds: f32,
    pub first_damage: u32,
    pub second_damage: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssaPhasePrismSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub duration_seconds: f32,
    pub swap_duration_unit_per_second: f32,
    pub damage: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FalconCarry {
    pub attributes: SkillCastingAttributes,
    pub carry_ally_duration: f32,
    pub carry_owner_duration: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FalconAttack {
    pub attributes: SkillCastingAttributes,
    pub damage: u32,
    pub slow: Percentage,
    pub duration_in_seconds: f32,
    pub slow_duration: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GazXplodiumChargeSkillConfigInner {
    pub missile_travel_duration_seconds: f32,
    pub detonation_duration: f32,
    pub damage: u32,
    pub stun_duration_seconds: f32,
    pub explosion_area: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GazXplodiumChargeSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub inner: GazXplodiumChargeSkillConfigInner,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GazTurretSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub turret: CharAttributes,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GazBarricadeSkillConfig {
    pub attributes: SkillCastingAttributes,
    pub char_attrs: CharAttributes,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
