use crate::components::controller::SkillKey;
use crate::components::skills::skill::Skills;
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusInAreaComponent, RemoveStatusComponent,
};
use crate::components::{ApplyForceComponent, AreaAttackComponent, AttackComponent};
use crate::consts::{JobId, MonsterId};
use crate::video::{DynamicVertexArray, GlTexture};
use crate::{DeltaTime, ElapsedTime, MapRenderData, RenderMatrices, Shaders, SpriteResource};
use nphysics2d::object::DefaultColliderHandle;
use std::collections::HashMap;
use std::time::Instant;

pub mod atk_calc;
pub mod camera_system;
pub mod char_state_sys;
pub mod input_sys;
pub mod input_to_next_action;
pub mod minion_ai_sys;
pub mod next_action_applier_sys;
pub mod phys;
pub mod render;
pub mod render_sys;
pub mod skill_sys;
pub mod ui;

pub struct EffectSprites {
    pub torch: SpriteResource,
    pub fire_wall: SpriteResource,
    pub fire_ball: SpriteResource,
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Sex {
    Male,
    Female,
}

pub struct Sprites {
    pub cursors: SpriteResource,
    pub numbers: GlTexture,
    pub character_sprites: HashMap<JobId, [SpriteResource; 2]>,
    pub mounted_character_sprites: HashMap<JobId, [SpriteResource; 2]>,
    pub head_sprites: [Vec<SpriteResource>; 2],
    pub monster_sprites: HashMap<MonsterId, SpriteResource>,
    pub effect_sprites: EffectSprites,
}

pub struct Texts {
    pub skill_name_texts: HashMap<Skills, GlTexture>,
    pub skill_key_texts: HashMap<SkillKey, GlTexture>,
    pub custom_texts: HashMap<String, GlTexture>,
    pub attack_absorbed: GlTexture,
    pub attack_blocked: GlTexture,
    pub minus: GlTexture,
    pub plus: GlTexture,
}

pub struct AssetResources {
    pub sprites: Sprites,
    pub shaders: Shaders,
    pub texts: Texts,
    pub skill_icons: HashMap<Skills, GlTexture>,
    pub status_icons: HashMap<&'static str, GlTexture>,
}

pub struct SystemVariables {
    pub assets: AssetResources,
    pub tick: u64,
    /// seconds the last frame required
    pub dt: DeltaTime,
    /// extract from the struct?
    pub time: ElapsedTime,
    pub matrices: RenderMatrices,
    pub map_render_data: MapRenderData,
    pub attacks: Vec<AttackComponent>,
    pub area_attacks: Vec<AreaAttackComponent>,
    pub pushes: Vec<ApplyForceComponent>,
    pub apply_statuses: Vec<ApplyStatusComponent>,
    pub apply_area_statuses: Vec<ApplyStatusInAreaComponent>,
    pub remove_statuses: Vec<RemoveStatusComponent>,
    // Todo: put it into the new Graphic module if it is ready
    pub str_effect_vao: DynamicVertexArray,
}

#[derive(Debug)]
pub struct Collision {
    pub character_coll_handle: DefaultColliderHandle,
    pub other_coll_handle: DefaultColliderHandle,
}

#[derive(Debug)]
pub struct CollisionsFromPrevFrame {
    pub collisions: HashMap<(DefaultColliderHandle, DefaultColliderHandle), Collision>,
}

impl CollisionsFromPrevFrame {
    pub fn remove_collider_handle(&mut self, collider_handle: DefaultColliderHandle) {
        self.collisions.retain(|(coll_1, coll_2), _collision| {
            *coll_1 != collider_handle && *coll_2 != collider_handle
        });
    }
}

#[derive(Clone)]
pub struct SystemFrameDurationsFrame {
    pub min: u32,
    pub max: u32,
    pub avg: u32,
}

#[derive(Clone)]
pub struct SystemFrameDurations(pub HashMap<&'static str, SystemFrameDurationsFrame>);

impl SystemFrameDurations {
    pub fn system_finished(&mut self, started: Instant, name: &'static str) {
        let duration = Instant::now().duration_since(started).as_micros() as u32;
        let mut durs = self.0.entry(name).or_insert(SystemFrameDurationsFrame {
            min: 100_000,
            max: 0,
            avg: 0,
        });
        durs.min = durs.min.min(duration);
        durs.max = durs.max.max(duration);
        durs.avg = (durs.avg + duration) / 2;
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
