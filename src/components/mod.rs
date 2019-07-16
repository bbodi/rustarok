extern crate rand;

use crate::cam::Camera;
use websocket::stream::sync::TcpStream;
use std::sync::Mutex;
use nalgebra::{Point3, Vector3, Vector2, Point2};
use std::collections::HashSet;
use sdl2::keyboard::Scancode;
use crate::{Tick, LIVING_COLLISION_GROUP, STATIC_MODELS_COLLISION_GROUP, ActionIndex, PhysicsWorld, ElapsedTime};
use specs::prelude::*;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{ColliderDesc, RigidBodyDesc};
use ncollide2d::world::CollisionGroups;
use rand::Rng;
use crate::components::skill::Skills;

pub mod char;
pub mod controller;
pub mod skill;

#[derive(Component)]
pub struct BrowserClient {
    pub websocket: Mutex<websocket::sync::Client<TcpStream>>,
    pub offscreen: Vec<u8>,
    pub ping: u16,
}

#[derive(Component)]
pub struct FlyingNumberComponent {
    pub value: u32,
    pub color: [f32; 3],
    pub start_pos: Point2<f32>,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
    pub duration: f32,
}

pub enum FlyingNumberType {
    Damage,
    Heal,
    Normal,
    Mana,
    Crit,
}

impl FlyingNumberComponent {
    pub fn new(typ: FlyingNumberType,
               value: u32,
               duration: f32,
               start_pos: Point2<f32>,
               sys_time: ElapsedTime) -> FlyingNumberComponent {
        FlyingNumberComponent {
            value,
            color: match typ {
                FlyingNumberType::Damage => [1.0, 0.0, 0.0],
                FlyingNumberType::Heal => [0.0, 1.0, 0.0],
                FlyingNumberType::Normal => [1.0, 1.0, 1.0],
                FlyingNumberType::Mana => [0.0, 0.0, 1.0],
                FlyingNumberType::Crit => [1.0, 1.0, 1.0]
            },
            start_pos,
            start_time: sys_time,
            die_at: sys_time.add_seconds(duration),
            duration,
        }
    }
}

pub enum AttackType {
    Basic,
    Skill(Skills)
}

#[derive(Component)]
pub struct AttackComponent {
    pub src_entity: Entity,
    pub dst_entity: Entity,
    pub typ: AttackType,
}