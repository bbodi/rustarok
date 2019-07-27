extern crate rand;

use websocket::stream::sync::TcpStream;
use std::sync::Mutex;
use nalgebra::{Vector2};
use crate::{ElapsedTime};
use specs::prelude::*;
use crate::components::controller::WorldCoords;
use nphysics2d::object::BodyHandle;
use crate::components::skills::skill::Skills;

pub mod char;
pub mod controller;
pub mod status;
pub mod skills;

#[derive(Component)]
pub struct BrowserClient {
    pub websocket: Mutex<websocket::sync::Client<TcpStream>>,
    pub offscreen: Vec<u8>,
    pub ping: u16,
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
    pub effect: String /*StrEffect*/,
    pub pos: WorldCoords,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
    pub duration: ElapsedTime,
}

pub enum FlyingNumberType {
    Damage,
    Poison,
    Heal,
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
            FlyingNumberType::Crit => [1.0, 1.0, 1.0]
        }
    }
}

impl FlyingNumberComponent {
    pub fn new(typ: FlyingNumberType,
               value: u32,
               target_entity_id: Entity,
               duration: f32,
               start_pos: Vector2<f32>,
               sys_time: ElapsedTime) -> FlyingNumberComponent {
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

pub enum AttackType {
    Basic,
    Skill(Skills),
}

pub struct AttackComponent {
    pub src_entity: Entity,
    pub dst_entity: Entity,
    pub typ: AttackType,
}

pub struct ApplyForceComponent {
    pub src_entity: Entity,
    pub dst_entity: Entity,
    pub force: Vector2<f32>,
    pub body_handle: BodyHandle,
    pub duration: f32,
}