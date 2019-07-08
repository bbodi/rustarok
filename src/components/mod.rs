extern crate rand;

use crate::cam::Camera;
use websocket::stream::sync::TcpStream;
use std::sync::Mutex;
use nalgebra::{Point3, Vector3, Vector2, Point2};
use std::collections::HashSet;
use sdl2::keyboard::Scancode;
use crate::{Tick, LIVING_COLLISION_GROUP, STATIC_MODELS_COLLISION_GROUP, ActionIndex, PhysicsWorld};
use specs::prelude::*;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{ColliderDesc, RigidBodyDesc};
use ncollide2d::world::CollisionGroups;
use rand::Rng;

pub mod char;
pub mod controller;

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
    pub start_tick: Tick,
    pub duration: u16,
}

pub enum FlyingNumberType {
    Damage,
    Heal,
    Normal,
    Mana,
    Crit,
}

impl FlyingNumberComponent {
    pub fn new(typ: FlyingNumberType, value: u32, start_pos: Point2<f32>, tick: Tick) -> FlyingNumberComponent {
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
            start_tick: tick,
            duration: 40,
        }
    }
}