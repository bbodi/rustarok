extern crate rand;

use crate::components::controller::WorldCoords;
use crate::systems::sound_sys::SoundId;
use crate::ElapsedTime;
use nalgebra::{Isometry2, Vector2};
use nphysics2d::object::DefaultBodyHandle;
use specs::prelude::*;
use std::sync::Mutex;
use websocket::stream::sync::TcpStream;

pub mod char;
pub mod controller;
pub mod skills;
pub mod status;

#[derive(Component)]
pub struct BrowserClient {
    pub websocket: Mutex<websocket::sync::Client<TcpStream>>,
    pub offscreen: Vec<u8>,
    pub ping: u16,
}

impl Drop for BrowserClient {
    fn drop(&mut self) {
        log::info!("BrowserClient DROPPED");
    }
}

#[derive(Component)]
pub struct FlyingNumberComponent {
    pub value: u32,
    pub target_entity_id: Entity,
    pub src_entity_id: Entity,
    pub typ: FlyingNumberType,
    pub start_pos: Vector2<f32>,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
    pub duration: f32,
}

#[derive(Component)]
pub struct SoundEffectComponent {
    pub target_entity_id: Entity,
    pub sound_id: SoundId,
    pub pos: WorldCoords,
    pub start_time: ElapsedTime,
}

#[derive(Component)]
pub struct StrEffectComponent {
    pub effect: String, /*StrEffect*/
    pub pos: WorldCoords,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
    pub duration: ElapsedTime,
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
    Mana,
    Crit,
}

impl FlyingNumberType {
    pub fn color(
        &self,
        target_is_current_user: bool,
        target_is_friend: bool,
        damage_was_initiated_by_current_user: bool,
    ) -> [f32; 3] {
        match self {
            FlyingNumberType::Damage | FlyingNumberType::SubCombo => {
                if target_is_current_user {
                    [1.0, 0.0, 0.0]
                } else if target_is_friend {
                    [1.0, 0.0, 0.0] // [1.0, 0.55, 0.0] orange
                } else if damage_was_initiated_by_current_user {
                    [1.0, 1.0, 1.0]
                } else {
                    [1.0, 1.0, 1.0]
                    //                    [0.73, 0.73, 0.73] // simple damage by other, greyish
                }
            }
            FlyingNumberType::Combo { .. } => [0.9, 0.9, 0.15],
            FlyingNumberType::Heal => [0.0, 1.0, 0.0],
            FlyingNumberType::Poison => [0.55, 0.0, 0.55],
            FlyingNumberType::Mana => [0.0, 0.0, 1.0],
            FlyingNumberType::Crit => [1.0, 1.0, 1.0],
            FlyingNumberType::Block => [1.0, 1.0, 1.0],
            FlyingNumberType::Absorb => [1.0, 1.0, 1.0],
        }
    }
}

impl FlyingNumberComponent {
    pub fn new(
        typ: FlyingNumberType,
        value: u32,
        src_entity_id: Entity,
        target_entity_id: Entity,
        duration: f32,
        start_pos: Vector2<f32>,
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
pub enum AttackType {
    Basic(u32),
    SpellDamage(u32),
    Heal(u32),
    Poison(u32),
}

#[derive(Debug)]
pub struct AttackComponent {
    pub src_entity: Entity,
    pub dst_entity: Entity,
    pub typ: AttackType,
}

pub struct AreaAttackComponent {
    pub area_shape: Box<dyn ncollide2d::shape::Shape<f32>>,
    pub area_isom: Isometry2<f32>,
    pub source_entity_id: Entity,
    pub typ: AttackType,
}

#[derive(Debug)]
pub struct ApplyForceComponent {
    pub src_entity: Entity,
    pub dst_entity: Entity,
    pub force: Vector2<f32>,
    pub body_handle: DefaultBodyHandle,
    pub duration: f32,
}
