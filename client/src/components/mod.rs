use nalgebra::Isometry2;
use nphysics2d::object::DefaultBodyHandle;
use specs::prelude::*;

use crate::audio::sound_sys::SoundId;
use crate::components::char::ActionPlayMode;
use crate::effect::StrEffectId;
use crate::LocalTime;
use rustarok_common::common::Vec2;
use rustarok_common::components::char::LocalCharEntityId;

pub mod char;
pub mod controller;
pub mod skills;
pub mod status;

#[derive(Component)]
pub struct FlyingNumberComponent {
    pub value: u32,
    pub target_entity_id: LocalCharEntityId,
    pub src_entity_id: LocalCharEntityId,
    pub typ: FlyingNumberType,
    pub start_pos: Vec2,
    pub start_time: LocalTime,
    pub die_at: LocalTime,
    pub duration_millis: u32,
}

#[derive(Component)]
pub struct SoundEffectComponent {
    pub target_entity_id: LocalCharEntityId,
    pub sound_id: SoundId,
    pub pos: Vec2,
    pub start_time: LocalTime,
}

#[derive(Component)]
pub struct StrEffectComponent {
    pub effect_id: StrEffectId,
    pub pos: Vec2,
    pub start_time: LocalTime,
    pub die_at: Option<LocalTime>,
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
        src_entity_id: LocalCharEntityId,
        target_entity_id: LocalCharEntityId,
        duration: u32,
        start_pos: Vec2,
        sys_time: LocalTime,
    ) -> FlyingNumberComponent {
        FlyingNumberComponent {
            value,
            typ,
            target_entity_id,
            src_entity_id,
            start_pos,
            start_time: sys_time,
            die_at: sys_time.add_millis(duration),
            duration_millis: duration,
        }
    }
}
