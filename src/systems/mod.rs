use std::time::Instant;
use crate::{Shaders, SpriteResource, Tick, RenderMatrices, MapRenderData, DeltaTime, ElapsedTime};
use std::collections::HashMap;
use crate::video::GlTexture;
use specs::Entity;
use crate::consts::{JobId, MonsterId};
use crate::components::controller::WorldCoords;
use nphysics2d::object::ColliderHandle;

pub mod input;
pub mod phys;
pub mod render;
pub mod ui;
pub mod control_sys;
pub mod skill_sys;
pub mod char_state_sys;
pub mod atk_calc;


pub struct EffectSprites {
    pub torch: SpriteResource,
    pub fire_wall: SpriteResource,
    pub fire_ball: SpriteResource,
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Sex {
    Male,
    Female
}

pub struct Sprites {
    pub cursors: SpriteResource,
    pub numbers: GlTexture,
    pub character_sprites: HashMap<JobId, [SpriteResource; 2]>,
    pub head_sprites: [Vec<SpriteResource>; 2],
    pub monster_sprites: HashMap<MonsterId, SpriteResource>,
    pub effect_sprites: EffectSprites,
}

pub struct SystemVariables {
    pub sprites: Sprites,
    pub shaders: Shaders,
    pub tick: Tick,
    pub dt: DeltaTime,
    pub time: ElapsedTime, // extract from the struct?
    pub matrices: RenderMatrices,
    pub map_render_data: MapRenderData,
}

pub struct Collision {
    pub character_coll_handle: ColliderHandle,
    pub other_coll_handle: ColliderHandle,
}

pub struct CollisionsFromPrevFrame {
    pub collisions: Vec<Collision>,
}

pub struct SystemFrameDurations(pub HashMap<&'static str, u32>);

impl SystemFrameDurations {
    pub fn system_finished(&mut self, started: Instant, name: &'static str) {
        let duration = Instant::now().duration_since(started).as_millis() as u32;
        self.0.insert(name, duration);
    }

    pub fn start_measurement(&mut self, name: &'static str) -> SystemStopwatch {
        SystemStopwatch::new(name, self)
    }
}

pub struct SystemStopwatch<'a> {
    // let now_ms = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    started: Instant,
    name: &'static str,
    times: &'a mut SystemFrameDurations,
}

impl<'a> SystemStopwatch<'a> {
    pub fn new(name: &'static str, times: &'a mut SystemFrameDurations) -> SystemStopwatch<'a> {
        SystemStopwatch {
            started: Instant::now(),
            name,
            times,
        }
    }
}

impl<'a> Drop for SystemStopwatch<'a> {
    fn drop(&mut self) {
        self.times.system_finished(self.started, self.name);
    }
}