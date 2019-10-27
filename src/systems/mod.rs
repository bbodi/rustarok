use crate::asset::str::StrFile;
use crate::asset::texture::{TextureId, DUMMY_TEXTURE_ID_FOR_TEST};
use crate::common::Mat4;
use crate::components::char::CharState;
use crate::components::controller::CharEntityId;
use crate::components::skills::skills::{FinishCast, Skills};
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusInAreaComponent, RemoveStatusComponent,
};
use crate::components::{
    ApplyForceComponent, AreaAttackComponent, HpModificationRequest, HpModificationResult,
};
use crate::consts::{JobId, JobSpriteId, MonsterId};
use crate::runtime_assets::audio::Sounds;
use crate::runtime_assets::graphic::Texts;
use crate::video::{ortho, VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::{
    get_current_ms, DeltaTime, ElapsedTime, SpriteResource, MAX_SECONDS_ALLOWED_FOR_SINGLE_FRAME,
};
use nphysics2d::object::DefaultColliderHandle;
use std::collections::HashMap;
use std::time::{Instant, SystemTime};

pub mod atk_calc;
pub mod camera_system;
pub mod char_state_sys;
pub mod console_commands;
pub mod console_system;
pub mod falcon_ai_sys;
pub mod frame_end_system;
pub mod input_sys;
pub mod input_sys_scancodes;
pub mod input_to_next_action;
pub mod minion_ai_sys;
pub mod next_action_applier_sys;
pub mod phys;
pub mod render;
pub mod render_sys;
pub mod skill_sys;
pub mod sound_sys;
pub mod spawn_entity_system;
pub mod turret_ai_sys;
pub mod ui;

pub struct EffectSprites {
    pub torch: SpriteResource,
    pub fire_wall: SpriteResource,
    pub fire_ball: SpriteResource,
    pub plasma: SpriteResource,
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Sex {
    Male,
    Female,
}

pub struct Sprites {
    pub cursors: SpriteResource,
    pub numbers: TextureId,
    pub magic_target: TextureId,
    pub fire_particle: TextureId,
    pub clock: TextureId,
    pub ginseng_bullet: SpriteResource,
    pub exoskeleton: SpriteResource,
    pub arrow: SpriteResource,
    pub falcon: SpriteResource,
    pub stun: SpriteResource,
    pub timefont: SpriteResource,
    // TODO: make it array
    pub character_sprites: HashMap<JobSpriteId, [[SpriteResource; 2]; 2]>,
    pub mounted_character_sprites: HashMap<JobId, [SpriteResource; 2]>,
    pub head_sprites: [Vec<SpriteResource>; 2],
    pub monster_sprites: HashMap<MonsterId, SpriteResource>,
    pub effect_sprites: EffectSprites,
}

impl Sprites {
    pub fn new_for_test() -> Sprites {
        Sprites {
            cursors: SpriteResource::new_for_test(),
            numbers: DUMMY_TEXTURE_ID_FOR_TEST,
            magic_target: DUMMY_TEXTURE_ID_FOR_TEST,
            fire_particle: DUMMY_TEXTURE_ID_FOR_TEST,
            clock: DUMMY_TEXTURE_ID_FOR_TEST,
            ginseng_bullet: SpriteResource::new_for_test(),
            exoskeleton: SpriteResource::new_for_test(),
            arrow: SpriteResource::new_for_test(),
            falcon: SpriteResource::new_for_test(),
            stun: SpriteResource::new_for_test(),
            timefont: SpriteResource::new_for_test(),
            character_sprites: HashMap::new(),
            mounted_character_sprites: HashMap::new(),
            head_sprites: [vec![], vec![]],
            monster_sprites: HashMap::new(),
            effect_sprites: EffectSprites {
                torch: SpriteResource::new_for_test(),
                fire_wall: SpriteResource::new_for_test(),
                fire_ball: SpriteResource::new_for_test(),
                plasma: SpriteResource::new_for_test(),
            },
        }
    }
}

pub struct AssetResources {
    pub sprites: Sprites,
    pub texts: Texts,
    pub skill_icons: HashMap<Skills, TextureId>,
    pub status_icons: HashMap<&'static str, TextureId>,
    pub sounds: Sounds,
}

pub struct RenderMatrices {
    pub projection: Mat4,
    pub ortho: Mat4,
}

impl RenderMatrices {
    pub fn new(fov: f32) -> RenderMatrices {
        RenderMatrices {
            projection: Mat4::new_perspective(
                VIDEO_WIDTH as f32 / VIDEO_HEIGHT as f32,
                fov,
                0.1f32,
                1000.0f32,
            ),
            ortho: ortho(0.0, VIDEO_WIDTH as f32, VIDEO_HEIGHT as f32, 0.0, -1.0, 1.0),
        }
    }
}

#[derive(Debug)]
pub enum SystemEvent {
    CharStatusChange(u64, CharEntityId, CharState, CharState),
    HpModification {
        timestamp: u64,
        src: CharEntityId,
        dst: CharEntityId,
        result: HpModificationResult,
    },
}

pub struct SystemVariables {
    pub assets: AssetResources,
    pub tick: u64,
    pub last_tick_time: u64,
    /// seconds the last frame required
    pub dt: DeltaTime,
    /// extract from the struct?
    pub time: ElapsedTime,
    pub matrices: RenderMatrices,
    pub hp_mod_requests: Vec<HpModificationRequest>,
    pub area_hp_mod_requests: Vec<AreaAttackComponent>,
    pub pushes: Vec<ApplyForceComponent>,
    pub apply_statuses: Vec<ApplyStatusComponent>,
    pub just_finished_skill_casts: Vec<FinishCast>,
    pub apply_area_statuses: Vec<ApplyStatusInAreaComponent>,
    pub remove_statuses: Vec<RemoveStatusComponent>,
    pub str_effects: Vec<StrFile>,
    pub fix_dt_for_test: f32,
}

impl SystemVariables {
    pub fn new(
        sprites: Sprites,
        texts: Texts,
        render_matrices: RenderMatrices,
        status_icons: HashMap<&'static str, TextureId>,
        skill_icons: HashMap<Skills, TextureId>,
        str_effects: Vec<StrFile>,
        sounds: Sounds,
        fix_dt_for_test: f32,
    ) -> SystemVariables {
        SystemVariables {
            assets: AssetResources {
                sprites,
                texts,
                skill_icons,
                status_icons,
                sounds,
            },
            last_tick_time: get_current_ms(SystemTime::now()),
            tick: 1,
            dt: DeltaTime(0.0),
            time: ElapsedTime(0.0),
            matrices: render_matrices,
            hp_mod_requests: Vec::with_capacity(128),
            area_hp_mod_requests: Vec::with_capacity(128),
            pushes: Vec::with_capacity(128),
            apply_statuses: Vec::with_capacity(128),
            just_finished_skill_casts: Vec::with_capacity(128),
            apply_area_statuses: Vec::with_capacity(128),
            remove_statuses: Vec::with_capacity(128),
            str_effects,
            fix_dt_for_test,
        }
    }

    pub fn update_timers(&mut self, now_ms: u64) {
        let dt = if cfg!(test) {
            self.fix_dt_for_test
        } else {
            let dt = (now_ms - self.last_tick_time) as f32 / 1000.0;
            self.last_tick_time = now_ms;
            dt.min(MAX_SECONDS_ALLOWED_FOR_SINGLE_FRAME)
        };
        self.tick += 1;
        self.dt.0 = dt;
        self.time.0 += dt;
    }
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
