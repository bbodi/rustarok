use nalgebra::Isometry2;
use nphysics2d::object::DefaultBodyHandle;
use specs::prelude::*;
//use websocket::stream::sync::TcpStream;

use crate::audio::sound_sys::SoundId;
use crate::components::char::ActionPlayMode;
use crate::components::skills::basic_attack::WeaponType;
use crate::effect::StrEffectId;
use crate::systems::CharEntityId;
use crate::ElapsedTime;
use rustarok_common::common::Vec2;

pub mod char;
pub mod controller;
pub mod skills;
pub mod status;

#[derive(Component)]
pub struct FlyingNumberComponent {
    pub value: u32,
    pub target_entity_id: CharEntityId,
    pub src_entity_id: CharEntityId,
    pub typ: FlyingNumberType,
    pub start_pos: Vec2,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
    pub duration: f32,
}

#[derive(Component)]
pub struct SoundEffectComponent {
    pub target_entity_id: CharEntityId,
    pub sound_id: SoundId,
    pub pos: Vec2,
    pub start_time: ElapsedTime,
}

#[derive(Component)]
pub struct StrEffectComponent {
    pub effect_id: StrEffectId,
    pub pos: Vec2,
    pub start_time: ElapsedTime,
    pub die_at: Option<ElapsedTime>,
    pub play_mode: ActionPlayMode,
}

#[derive(Component)]
pub struct MinionComponent {
    pub fountain_up: bool,
}

pub enum FlyingNumberType {
    Damage,
    Combo {
        single_attack_damage: u32,
        attack_count: u8,
    },
    SubCombo,
    Poison,
    Heal,
    Block,
    Absorb,
}

impl FlyingNumberType {
    pub fn color(
        &self,
        target_is_current_user: bool,
        target_is_friend: bool,
        damage_was_initiated_by_current_user: bool,
    ) -> [u8; 3] {
        match self {
            FlyingNumberType::Damage | FlyingNumberType::SubCombo => {
                if target_is_current_user {
                    [255, 0, 0]
                } else if target_is_friend {
                    [255, 0, 0] // [1.0, 0.55, 0.0] orange
                } else if damage_was_initiated_by_current_user {
                    [255, 255, 255]
                } else {
                    [255, 255, 255]
                    //                    [0.73, 0.73, 0.73] // simple damage by other, greyish
                }
            }
            FlyingNumberType::Combo { .. } => [230, 230, 38],
            FlyingNumberType::Heal => [0, 255, 0],
            FlyingNumberType::Poison => [140, 0, 140],
            FlyingNumberType::Block => [255, 255, 255],
            FlyingNumberType::Absorb => [255, 255, 255],
        }
    }
}

impl FlyingNumberComponent {
    pub fn new(
        typ: FlyingNumberType,
        value: u32,
        src_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        duration: f32,
        start_pos: Vec2,
        sys_time: ElapsedTime,
    ) -> FlyingNumberComponent {
        FlyingNumberComponent {
            value,
            typ,
            target_entity_id,
            src_entity_id,
            start_pos,
            start_time: sys_time,
            die_at: sys_time.add_seconds(duration),
            duration,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum DamageDisplayType {
    SingleNumber,
    Combo(u8),
}

#[derive(Clone, Copy, Debug)]
pub enum HpModificationType {
    BasicDamage(u32, DamageDisplayType, WeaponType),
    SpellDamage(u32, DamageDisplayType),
    Heal(u32),
    Poison(u32),
}

#[derive(Debug)]
pub struct HpModificationRequest {
    pub src_entity: CharEntityId,
    pub dst_entity: CharEntityId,
    pub typ: HpModificationType,
}

impl HpModificationRequest {
    pub fn allow(self, dmg: u32) -> HpModificationResult {
        HpModificationResult {
            src_entity: self.src_entity,
            dst_entity: self.dst_entity,
            typ: HpModificationResultType::Ok(match self.typ {
                HpModificationType::BasicDamage(_, display_type, weapon_type) => {
                    HpModificationType::BasicDamage(dmg, display_type, weapon_type)
                }
                HpModificationType::SpellDamage(_, display_type) => {
                    HpModificationType::SpellDamage(dmg, display_type)
                }
                HpModificationType::Heal(_) => HpModificationType::Heal(dmg),
                HpModificationType::Poison(_) => HpModificationType::Poison(dmg),
            }),
        }
    }

    pub fn blocked(self) -> HpModificationResult {
        HpModificationResult {
            src_entity: self.src_entity,
            dst_entity: self.dst_entity,
            typ: HpModificationResultType::Blocked,
        }
    }
}

#[derive(Debug)]
pub struct HpModificationResult {
    pub src_entity: CharEntityId,
    pub dst_entity: CharEntityId,
    pub typ: HpModificationResultType,
}

impl HpModificationResult {
    pub fn absorbed(self) -> HpModificationResult {
        HpModificationResult {
            src_entity: self.src_entity,
            dst_entity: self.dst_entity,
            typ: HpModificationResultType::Absorbed,
        }
    }
}

#[derive(Debug)]
pub enum HpModificationResultType {
    Ok(HpModificationType),
    Blocked,
    Absorbed,
}

// TODO: be static types for Cuboid area attack components, Circle, etc
pub struct AreaAttackComponent {
    pub area_shape: Box<dyn ncollide2d::shape::Shape<f32>>,
    pub area_isom: Isometry2<f32>,
    pub source_entity_id: CharEntityId,
    pub typ: HpModificationType,
    pub except: Option<CharEntityId>,
}

#[derive(Debug)]
pub struct ApplyForceComponent {
    pub src_entity: CharEntityId,
    pub dst_entity: CharEntityId,
    pub force: Vec2,
    pub body_handle: DefaultBodyHandle,
    pub duration: f32,
}
