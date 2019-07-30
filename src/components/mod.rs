extern crate rand;

use crate::components::controller::WorldCoords;
use crate::ElapsedTime;
use nalgebra::{Isometry2, Vector2};
use nphysics2d::object::BodyHandle;
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
    pub typ: FlyingNumberType,
    pub start_pos: Vector2<f32>,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
    pub duration: f32,
}

#[derive(Component)]
pub struct StrEffectComponent {
    pub effect: String, /*StrEffect*/
    pub pos: WorldCoords,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
    pub duration: ElapsedTime,
}

pub enum FlyingNumberType {
    Damage,
    Poison,
    Heal,
    Block,
    Absorb,
    Mana,
    Crit,
}

impl FlyingNumberType {
    pub fn color(&self, target_is_current_user: bool) -> [f32; 3] {
        match self {
            FlyingNumberType::Damage => {
                if target_is_current_user {
                    [1.0, 0.0, 0.0]
                } else {
                    [1.0, 1.0, 1.0]
                }
            }
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
        target_entity_id: Entity,
        duration: f32,
        start_pos: Vector2<f32>,
        sys_time: ElapsedTime,
    ) -> FlyingNumberComponent {
        FlyingNumberComponent {
            value,
            typ,
            target_entity_id,
            start_pos,
            start_time: sys_time,
            die_at: sys_time.add_seconds(duration),
            duration,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum AttackType {
    Basic(u32),
    SpellDamage(u32),
    Heal(u32),
    Poison(u32),
}

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

pub struct ApplyForceComponent {
    pub src_entity: Entity,
    pub dst_entity: Entity,
    pub force: Vector2<f32>,
    pub body_handle: BodyHandle,
    pub duration: f32,
}
