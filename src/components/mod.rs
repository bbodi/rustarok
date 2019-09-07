extern crate rand;

use crate::components::controller::{CharEntityId, WorldCoords};
use crate::effect::StrEffectId;
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
    websocket: Mutex<websocket::sync::Client<TcpStream>>,
    pub ping: u16,
    pub state: usize,
    pub sum_sent_bytes: u64,
    pub current_bytes_per_second: u32,
    pub prev_bytes_per_second: u32,
}

impl Drop for BrowserClient {
    fn drop(&mut self) {
        log::info!("BrowserClient DROPPED");
    }
}

impl BrowserClient {
    pub fn new(websocket: websocket::sync::Client<TcpStream>) -> BrowserClient {
        BrowserClient {
            websocket: Mutex::new(websocket),
            ping: 0,
            state: 0,
            sum_sent_bytes: 0,
            current_bytes_per_second: 0,
            prev_bytes_per_second: 0,
        }
    }

    pub fn send_message(&mut self, buf: &Vec<u8>) {
        self.sum_sent_bytes += buf.len() as u64;
        self.current_bytes_per_second += buf.len() as u32;
        let msg = websocket::Message::binary(buf.as_slice());
        let _ = self.websocket.lock().unwrap().send_message(&msg);
    }

    pub fn set_ping(&mut self, ping: u128) {
        self.ping = ping as u16
    }

    pub fn send_ping(&mut self, buf: &[u8]) {
        self.sum_sent_bytes += buf.len() as u64;
        self.current_bytes_per_second += buf.len() as u32;
        let msg = websocket::Message::ping(buf);
        let _ = self.websocket.lock().unwrap().send_message(&msg);
    }

    pub fn reset_byte_per_second(&mut self) {
        self.prev_bytes_per_second = self.current_bytes_per_second;
        self.current_bytes_per_second = 0;
    }

    pub fn receive(
        &mut self,
    ) -> websocket::result::WebSocketResult<websocket::message::OwnedMessage> {
        self.websocket.lock().unwrap().recv_message()
    }
}

#[derive(Component)]
pub struct FlyingNumberComponent {
    pub value: u32,
    pub target_entity_id: CharEntityId,
    pub src_entity_id: CharEntityId,
    pub typ: FlyingNumberType,
    pub start_pos: Vector2<f32>,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
    pub duration: f32,
}

#[derive(Component)]
pub struct SoundEffectComponent {
    pub target_entity_id: CharEntityId,
    pub sound_id: SoundId,
    pub pos: WorldCoords,
    pub start_time: ElapsedTime,
}

#[derive(Component)]
pub struct StrEffectComponent {
    pub effect_id: StrEffectId,
    pub pos: WorldCoords,
    pub start_time: ElapsedTime,
    pub die_at: ElapsedTime,
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
            FlyingNumberType::Mana => [0, 0, 255],
            FlyingNumberType::Crit => [255, 255, 255],
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
    pub src_entity: CharEntityId,
    pub dst_entity: CharEntityId,
    pub typ: AttackType,
}

pub struct AreaAttackComponent {
    pub area_shape: Box<dyn ncollide2d::shape::Shape<f32>>,
    pub area_isom: Isometry2<f32>,
    pub source_entity_id: CharEntityId,
    pub typ: AttackType,
}

#[derive(Debug)]
pub struct ApplyForceComponent {
    pub src_entity: CharEntityId,
    pub dst_entity: CharEntityId,
    pub force: Vector2<f32>,
    pub body_handle: DefaultBodyHandle,
    pub duration: f32,
}
