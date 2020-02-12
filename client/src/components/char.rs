use std::collections::HashMap;

use ncollide2d::pipeline::CollisionGroups;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{
    BodyPartHandle, BodyStatus, ColliderDesc, DefaultBodyHandle, DefaultColliderHandle,
    RigidBodyDesc,
};
use rustarok_common::common::{v2, EngineTime, Mat4, Vec2};
use serde::Deserialize;
use serde::Serialize;
use specs::prelude::*;

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::controller::{
    CameraComponent, HumanInputComponent, LocalPlayerController, SkillKey,
};
use crate::components::skills::skills::Skills;
use crate::components::status::status::Statuses;
use crate::grf::SpriteResource;
use crate::render::render_command::RenderCommandCollector;
use crate::runtime_assets::ecs::create_ecs_world;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::{Sprites, SystemVariables};
use crate::ElapsedTime;
use rand::Rng;
use rustarok_common::attack::{BasicAttackType, WeaponType};
use rustarok_common::char_attr::{BonusDurations, CharAttributes, CharAttributesBonuses};
use rustarok_common::components::char::{
    create_common_player_entity, AuthorizedCharStateComponent, CharDir, CharEntityId, CharOutlook,
    CharState, CharType, CollisionGroup, ControllerEntityId, EntityTarget, JobId, MonsterId,
    ServerEntityId, Sex, StaticCharDataComponent, Team,
};
use rustarok_common::components::job_ids::JobSpriteId;
use rustarok_common::components::snapshot::CharSnapshot;
use rustarok_common::config::CommonConfigs;

#[derive(Component, Debug)]
pub struct HasServerIdComponent {
    pub server_id: ServerEntityId,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum CharActionIndex {
    Idle = 0,
    Walking = 8,
    Sitting = 16,
    PickingItem = 24,
    StandBy = 32,
    Attacking1 = 40,
    ReceivingDamage = 48,
    Freeze1 = 56,
    Dead = 65,
    Freeze2 = 72,
    Attacking2 = 80,
    Attacking3 = 88,
    CastingSpell = 96,
}

#[derive(Clone, Copy)]
pub enum MonsterActionIndex {
    Idle = 0,
    Walking = 8,
    Attack = 16,
    ReceivingDamage = 24,
    Die = 32,
}

pub struct CharPhysicsEntityBuilder<'a> {
    pos2d: Vec2,
    self_group: CollisionGroup,
    collider_shape: ShapeHandle<f32>,
    blacklist_groups: &'a [CollisionGroup],
    body_status: BodyStatus,
}

impl<'a> CharPhysicsEntityBuilder<'a> {
    pub fn new(pos2d: Vec2) -> CharPhysicsEntityBuilder<'a> {
        CharPhysicsEntityBuilder {
            pos2d,
            self_group: CollisionGroup::StaticModel,
            collider_shape: ShapeHandle::new(ncollide2d::shape::Ball::new(1.0)),
            blacklist_groups: &[],
            body_status: BodyStatus::Dynamic,
        }
    }

    pub fn collision_group(mut self, self_group: CollisionGroup) -> CharPhysicsEntityBuilder<'a> {
        self.self_group = self_group;
        self.blacklist_groups = match self_group {
            CollisionGroup::Guard => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::StaticModel,
                CollisionGroup::LeftPlayer,
                CollisionGroup::RightPlayer,
                CollisionGroup::Guard,
                CollisionGroup::SkillArea,
                CollisionGroup::Turret,
                CollisionGroup::NeutralPlayerPlayer,
                CollisionGroup::LeftBarricade,
                CollisionGroup::RightBarricade,
            ],
            CollisionGroup::StaticModel => panic!(),
            CollisionGroup::LeftPlayer | CollisionGroup::RightPlayer => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
            ],
            CollisionGroup::NonCollidablePlayer => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::StaticModel,
                CollisionGroup::LeftPlayer,
                CollisionGroup::RightPlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
                CollisionGroup::NeutralPlayerPlayer,
            ],
            CollisionGroup::Minion => &[
                CollisionGroup::LeftPlayer,
                CollisionGroup::RightPlayer,
                CollisionGroup::StaticModel,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Turret,
                CollisionGroup::NeutralPlayerPlayer,
            ],
            CollisionGroup::SkillArea => panic!(),
            CollisionGroup::Turret => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::StaticModel,
                CollisionGroup::LeftPlayer,
                CollisionGroup::RightPlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
                CollisionGroup::NeutralPlayerPlayer,
            ],
            CollisionGroup::NeutralPlayerPlayer => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
            ],
            CollisionGroup::LeftBarricade => &[
                CollisionGroup::LeftPlayer,
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
            ],
            CollisionGroup::RightBarricade => &[
                CollisionGroup::RightPlayer,
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
            ],
        };
        self
    }

    pub fn body_status(mut self, body_status: BodyStatus) -> CharPhysicsEntityBuilder<'a> {
        self.body_status = body_status;
        self
    }

    pub fn circle(mut self, radius: f32) -> CharPhysicsEntityBuilder<'a> {
        self.collider_shape = ShapeHandle::new(ncollide2d::shape::Ball::new(radius));
        self
    }

    pub fn rectangle(mut self, w: f32, h: f32) -> CharPhysicsEntityBuilder<'a> {
        self.collider_shape =
            ShapeHandle::new(ncollide2d::shape::Cuboid::new(v2(w / 2.0, h / 2.0)));
        self
    }
}

pub fn create_client_player_entity(
    world: &mut specs::World,
    name: String,
    job_id: JobId,
    pos: Vec2,
    team: Team,
    outlook: CharOutlook,
    server_id: ServerEntityId,
) -> CharEntityId {
    let base_attrs =
        CharAttributes::get_base_attributes(job_id, &world.read_resource::<CommonConfigs>())
            .clone();
    let builder = create_common_player_entity(world, job_id, pos, team, outlook.clone());
    return CharEntityId::from(
        builder
            .with(SpriteRenderDescriptorComponent::new())
            .with(HasServerIdComponent { server_id })
            .with(CharacterStateComponent::new(
                name,
                0.0,
                CharType::Player,
                outlook,
                job_id,
                team,
                base_attrs,
            ))
            .build(),
    );
}

pub fn create_client_barricade_entity(
    world: &mut specs::World,
    pos: Vec2,
    team: Team,
) -> CharEntityId {
    let base_attrs = CharAttributes::get_base_attributes(
        JobId::Barricade,
        &world.read_resource::<CommonConfigs>(),
    )
    .clone();

    let builder = create_common_player_entity(
        world,
        JobId::Barricade,
        pos,
        team,
        CharOutlook::Monster(MonsterId::Barricade),
    );

    return CharEntityId::from(
        builder
            .with(SpriteRenderDescriptorComponent::new())
            .with(CharacterStateComponent::new(
                "barricade".to_owned(),
                0.0,
                CharType::Minion,
                CharOutlook::Monster(MonsterId::Barricade),
                JobId::Barricade,
                team,
                base_attrs,
            ))
            .build(),
    );
}

pub fn create_client_dummy_entity(
    world: &mut specs::World,
    job_id: JobId,
    pos: Vec2,
) -> CharEntityId {
    let outlook = if job_id == JobId::HealingDummy {
        CharOutlook::Monster(MonsterId::GEFFEN_MAGE_6)
    } else {
        CharOutlook::Monster(MonsterId::Barricade)
    };
    let team = if job_id == JobId::HealingDummy {
        Team::AllyForAll
    } else {
        Team::EnemyForAll
    };

    let base_attrs =
        CharAttributes::get_base_attributes(job_id, &world.read_resource::<CommonConfigs>())
            .clone();
    let builder = create_common_player_entity(world, job_id, pos, team, outlook.clone());

    return CharEntityId::from(
        builder
            .with(SpriteRenderDescriptorComponent::new())
            .with(CharacterStateComponent::new(
                if job_id == JobId::HealingDummy {
                    "Healing Dummy".to_owned()
                } else {
                    "Target Dummy".to_owned()
                },
                0.0,
                CharType::Guard,
                outlook,
                JobId::HealingDummy,
                team,
                base_attrs,
            ))
            .with(NpcComponent)
            .build(),
    );
}

pub fn create_client_guard_entity(
    world: &mut specs::World,
    pos: Vec2,
    team: Team,
    y: f32,
) -> CharEntityId {
    let outlook = if team == Team::Left {
        CharOutlook::Monster(MonsterId::GEFFEN_MAGE_9) // blue
    } else {
        CharOutlook::Monster(MonsterId::GEFFEN_MAGE_12)
    };
    let base_attrs =
        CharAttributes::get_base_attributes(JobId::Guard, &world.read_resource::<CommonConfigs>())
            .clone();
    let builder = create_common_player_entity(world, JobId::Guard, pos, team, outlook.clone());

    return CharEntityId::from(
        builder
            .with(SpriteRenderDescriptorComponent::new())
            .with(CharacterStateComponent::new(
                "Guard".to_string(),
                y,
                CharType::Guard,
                outlook.clone(),
                JobId::Guard,
                team,
                base_attrs,
            ))
            .with(NpcComponent)
            .build(),
    );
}

pub fn create_client_minion_entity(
    world: &mut specs::World,
    pos: Vec2,
    team: Team,
) -> CharEntityId {
    let mut rng = rand::thread_rng();
    let sex = if rng.gen::<usize>() % 2 == 0 {
        Sex::Male
    } else {
        Sex::Female
    };

    let (job_id, job_sprite_id) = if rng.gen::<usize>() % 2 == 0 {
        (JobId::SWORDMAN, JobSpriteId::SWORDMAN)
    } else {
        (JobId::ARCHER, JobSpriteId::ARCHER)
    };
    let head_index = {
        let head_count = world
            .read_resource::<SystemVariables>()
            .assets
            .sprites
            .head_sprites[Sex::Male as usize]
            .len();
        rng.gen::<usize>() % head_count
    };
    let outlook = CharOutlook::Player {
        job_sprite_id,
        head_index,
        sex,
    };

    let base_attrs =
        CharAttributes::get_base_attributes(job_id, &world.read_resource::<CommonConfigs>())
            .clone();
    let builder = create_common_player_entity(world, job_id, pos, team, outlook.clone());

    return CharEntityId::from(
        builder
            .with(SpriteRenderDescriptorComponent::new())
            .with(NpcComponent)
            .with(CharacterStateComponent::new(
                "minion".to_owned(),
                0.0,
                CharType::Minion,
                outlook.clone(),
                job_id,
                team,
                base_attrs,
            ))
            .build(),
    );
}

pub struct CharacterEntityBuilder {
    char_id: CharEntityId,
    name: String,
    pub physics_handles: Option<(DefaultColliderHandle, DefaultBodyHandle)>,
}

impl CharacterEntityBuilder {
    pub fn new(char_id: CharEntityId, name: &str) -> CharacterEntityBuilder {
        CharacterEntityBuilder {
            char_id,
            name: name.to_owned(),
            physics_handles: None,
        }
    }

    pub fn insert_npc_component(self, updater: &LazyUpdate) -> CharacterEntityBuilder {
        updater.insert(self.char_id.into(), NpcComponent);
        self
    }
}

// radius = ComponentRadius * 0.5f32
#[derive(Eq, PartialEq, Hash)]
pub struct ComponentRadius(pub i32);

#[derive(Clone, Debug, PartialEq)]
pub struct CastingSkillData {
    pub target_area_pos: Option<Vec2>,
    pub char_to_skill_dir_when_casted: Vec2,
    pub target_entity: Option<CharEntityId>,
    pub cast_started: ElapsedTime,
    pub cast_ends: ElapsedTime,
    pub can_move: bool,
    pub skill: Skills,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClientCharState {
    CastingSkill(CastingSkillData),
}

unsafe impl Sync for ClientCharState {}

unsafe impl Send for ClientCharState {}

pub fn get_sprite_index(state: &CharState, is_monster: bool) -> usize {
    // TODO2
    match (state, is_monster) {
        (CharState::Idle, false) => CharActionIndex::Idle as usize,
        (CharState::Walking(_pos), false) => CharActionIndex::Walking as usize,
        (CharState::StandBy, false) => CharActionIndex::StandBy as usize,
        (CharState::Attacking { .. }, false) => CharActionIndex::Attacking3 as usize,
        (CharState::ReceivingDamage, false) => CharActionIndex::ReceivingDamage as usize,
        (CharState::Dead, false) => CharActionIndex::Dead as usize,
        //        (CharState::CastingSkill { .. }, false) => CharActionIndex::CastingSpell as usize,

        // monster
        (CharState::Idle, true) => MonsterActionIndex::Idle as usize,
        (CharState::Walking(_pos), true) => MonsterActionIndex::Walking as usize,
        (CharState::StandBy, true) => MonsterActionIndex::Idle as usize,
        (CharState::Attacking { .. }, true) => MonsterActionIndex::Attack as usize,
        (CharState::ReceivingDamage, true) => MonsterActionIndex::ReceivingDamage as usize,
        (CharState::Dead, true) => MonsterActionIndex::Die as usize,
        //        (CharState::CastingSkill { .. }, true) => MonsterActionIndex::Attack as usize,
    }
}

#[derive(Default, Debug)]
pub struct SpriteBoundingRect {
    pub bottom_left: [i32; 2],
    pub top_right: [i32; 2],
}

impl SpriteBoundingRect {
    pub fn merge(&mut self, other: &SpriteBoundingRect) {
        self.bottom_left[0] = self.bottom_left[0].min(other.bottom_left[0]);
        self.bottom_left[1] = self.bottom_left[1].max(other.bottom_left[1]);

        self.top_right[0] = self.top_right[0].max(other.top_right[0]);
        self.top_right[1] = self.top_right[1].min(other.top_right[1]);
    }
}

pub fn get_sprite_and_action_index<'a>(
    outlook: &CharOutlook,
    sprites: &'a Sprites,
    char_state: &CharState,
) -> (&'a SpriteResource, usize) {
    return match outlook {
        CharOutlook::Player {
            job_sprite_id,
            head_index: _,
            sex,
        } => {
            let sprites = &sprites.character_sprites;
            (
                // this function is used only for
                // getting animation duration information,
                // so color (the first array index) does not matter
                &sprites[&job_sprite_id][Team::Left as usize][*sex as usize],
                get_sprite_index(char_state, false),
            )
        }
        CharOutlook::Monster(monster_id) => (
            &sprites.monster_sprites[&monster_id],
            get_sprite_index(char_state, true),
        ),
    };
}

pub struct TurretControllerComponent;

impl Component for TurretControllerComponent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Component)]
pub struct TurretComponent {
    pub owner_entity_id: CharEntityId,
    pub preferred_target: Option<CharEntityId>,
}

pub struct NpcComponent;

impl Component for NpcComponent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

// TODO: extract attributes which won't change frame-by-frame (team, job id etc)
// TODO: extract everything which is not serializable
#[derive(Component)]
pub struct CharacterStateComponent {
    // characters also has names so it is possible to follow them with a camera
    pub name: String,
    y: f32,
    prev_state: CharState,
    // TODO: the whole Statuses struct needs for simulation but not for state representation. Extract the array from it for serialization
    pub statuses: Statuses,
    pub body_handle: DefaultBodyHandle,
    pub collider_handle: DefaultColliderHandle,
}

impl CharacterStateComponent {
    pub fn set_noncollidable(&self, physics_world: &mut PhysicEngine) {
        // TODO2
        //        if let Some(collider) = physics_world.colliders.get_mut(self.collider_handle) {
        //            let mut cg = collider.collision_groups().clone();
        //            cg.modify_membership(self.team.get_collision_group() as usize, false);
        //            cg.modify_membership(CollisionGroup::NonCollidablePlayer as usize, true);
        //            collider.set_collision_groups(cg);
        //        }
        //        if let Some(body) = physics_world.bodies.get_mut(self.body_handle) {
        //            body.set_status(BodyStatus::Kinematic);
        //        }
    }

    pub fn set_collidable(&self, physics_world: &mut PhysicEngine) {
        // TODO2
        //        if let Some(collider) = physics_world.colliders.get_mut(self.collider_handle) {
        //            let mut cg = collider.collision_groups().clone();
        //            cg.modify_membership(self.team.get_collision_group() as usize, true);
        //            cg.modify_membership(CollisionGroup::NonCollidablePlayer as usize, false);
        //            collider.set_collision_groups(cg);
        //        }
        //        if let Some(body) = physics_world.bodies.get_mut(self.body_handle) {
        //            body.set_status(BodyStatus::Dynamic);
        //        }
    }

    pub fn new(
        name: String,
        y: f32,
        char_type: CharType,
        outlook: CharOutlook,
        job_id: JobId,
        team: Team,
        base_attrs: CharAttributes,
    ) -> CharacterStateComponent {
        let statuses = Statuses::new();
        let calculated_attribs = base_attrs;
        CharacterStateComponent {
            name,
            y,
            prev_state: CharState::Idle,
            statuses,
            // hack, remove these
            body_handle: DefaultBodyHandle::from_raw_parts(1, 2),
            collider_handle: DefaultBodyHandle::from_raw_parts(1, 2),
        }
    }

    pub fn recalc_attribs_based_on_statuses(&mut self, dev_configs: &CommonConfigs) {
        // TODO2
        //        let base_attributes = CharAttributes::get_base_attributes(self.job_id, dev_configs);
        //        let modifier_collector = self.statuses.calc_attributes();
        //        self.calculated_attribs = base_attributes.apply(modifier_collector);
        //
        //        self.attrib_bonuses = self
        //            .calculated_attribs
        //            .differences(&base_attributes, modifier_collector);
    }

    // for tests
    #[allow(dead_code)]
    pub fn get_status_count(&self) -> usize {
        self.statuses.count()
    }

    pub fn update_statuses(
        &mut self,
        self_char_id: CharEntityId,
        sys_vars: &mut SystemVariables,
        time: &EngineTime,
        entities: &Entities,
        updater: &mut LazyUpdate,
        phyisics_world: &mut PhysicEngine,
        dev_configs: &CommonConfigs,
    ) {
        // TODO: refactor this
        //                 a hack so statuses and self can be mut at the same time
        let mut mut_statuses = std::mem::replace(&mut self.statuses, Statuses::new());
        // TODO2 status
        //        let bit_indices_of_changed_statuses = mut_statuses.update(
        //            self_char_id,
        //            self,
        //            phyisics_world,
        //            sys_vars,
        //            time,
        //            entities,
        //            updater,
        //        );
        //        std::mem::replace(&mut self.statuses, mut_statuses);
        //        if bit_indices_of_changed_statuses > 0 {
        //            self.statuses
        //                .remove_statuses(bit_indices_of_changed_statuses);
        //            self.recalc_attribs_based_on_statuses(dev_configs);
        //            // TODO2
        //            //            log::trace!(
        //            //                "Status expired. Attributes({:?}): mod: {:?}, attribs: {:?}",
        //            //                self_char_id,
        //            //                self.attrib_bonuses(),
        //            //                self.calculated_attribs()
        //            //            );
        //        }
    }

    pub fn set_y(&mut self, y: f32) {
        self.y = y;
    }

    pub fn get_y(&self) -> f32 {
        self.y
    }

    pub fn state_type_has_changed(&self, state: &CharState) -> bool {
        return !self.prev_state.discriminant_eq(state);
    }

    pub fn save_prev_state(&mut self, state: &CharState) {
        self.prev_state = state.clone();
    }

    pub fn prev_state(&self) -> &CharState {
        &self.prev_state
    }

    pub fn went_from_casting_to_idle(&self, current_state: &CharState) -> bool {
        match current_state {
            CharState::Idle => match self.prev_state {
                // TODO2
                //                CharState::CastingSkill(_) => true,
                _ => false,
            },
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ActionPlayMode {
    Repeat,
    PlayThenHold,
    Once,
    Reverse,
    FixFrame(usize),
}

#[derive(Component)]
pub struct SpriteRenderDescriptorComponent {
    pub action_index: usize,
    pub fps_multiplier: f32,
    pub animation_started: ElapsedTime,
    pub forced_duration: Option<ElapsedTime>,
    pub direction: CharDir,
    /// duration of the current animation
    pub animation_ends_at: ElapsedTime,
}

impl SpriteRenderDescriptorComponent {
    pub fn new() -> SpriteRenderDescriptorComponent {
        SpriteRenderDescriptorComponent {
            action_index: CharActionIndex::Idle as usize,
            animation_started: ElapsedTime(0.0),
            animation_ends_at: ElapsedTime(0.0),
            forced_duration: None,
            direction: CharDir::South,
            fps_multiplier: 1.0,
        }
    }
}
