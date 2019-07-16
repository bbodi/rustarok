use ncollide2d::shape::ShapeHandle;
use nalgebra::{Vector2, Point2};
use nphysics2d::object::{ColliderDesc, RigidBodyDesc};
use ncollide2d::world::CollisionGroups;
use crate::{LIVING_COLLISION_GROUP, STATIC_MODELS_COLLISION_GROUP, PhysicsWorld, Tick, ActionIndex, ElapsedTime};
use specs::Entity;
use specs::prelude::*;
use crate::consts::{MonsterId, JobId};
use crate::systems::Sex;
use crate::components::skill::{SkillDescriptor};
use crate::systems::control_sys::CharacterControlSystem;
use std::sync::{Mutex, Arc};

pub fn create_char(
    ecs_world: &mut specs::world::World,
    pos2d: Point2<f32>,
    sex: Sex,
    job_id: JobId,
    head_index: usize,
    radius: i32,
) -> Entity {
    let entity_id = {
        let mut entity_builder = ecs_world.create_entity()
            .with(CharacterStateComponent::new());
        let entity_id = entity_builder.entity;
        entity_builder = entity_builder.with(PlayerSpriteComponent {
            job_id,
            head_index,
            sex,
            descr: SpriteRenderDescriptor {
                action_index: ActionIndex::Idle as usize,
                animation_started: ElapsedTime(0.0),
                forced_duration: None,
                direction: 0,
            },
        });
        entity_builder.build()
    };
    let mut storage = ecs_world.write_storage();
    let mut physics_world = &mut ecs_world.write_resource::<PhysicsWorld>();
    let physics_component = PhysicsComponent::new(physics_world, pos2d.coords, ComponentRadius(radius), entity_id);
    storage.insert(entity_id, physics_component).unwrap();
    return entity_id;
}

pub fn create_monster(
    ecs_world: &mut specs::world::World,
    pos2d: Point2<f32>,
    monster_id: MonsterId,
    radius: i32,
) -> Entity {
    let entity_id = {
        let mut entity_builder = ecs_world.create_entity()
            .with(CharacterStateComponent::new());
        let entity_id = entity_builder.entity;
        entity_builder = entity_builder.with(MonsterSpriteComponent {
            monster_id,
            descr: SpriteRenderDescriptor {
                action_index: 8,
                animation_started: ElapsedTime(0.0),
                forced_duration: None,
                direction: 0,
            },
        });
        entity_builder.build()
    };
    let mut storage = ecs_world.write_storage();
    let mut physics_world = &mut ecs_world.write_resource::<PhysicsWorld>();
    let physics_component = PhysicsComponent::new(physics_world, pos2d.coords, ComponentRadius(radius), entity_id);
    storage.insert(entity_id, physics_component).unwrap();
    return entity_id;
}


// radius = ComponentRadius * 0.5f32
#[derive(Eq, PartialEq, Hash)]
pub struct ComponentRadius(pub i32);

impl ComponentRadius {
    pub fn get(&self) -> f32 {
        self.0 as f32 * 0.5
    }
}

#[derive(Component)]
pub struct PhysicsComponent {
    pub radius: ComponentRadius,
    pub body_handle: nphysics2d::object::BodyHandle,
}

impl PhysicsComponent {
    pub fn new(
        world: &mut nphysics2d::world::World<f32>,
        pos: Vector2<f32>,
        radius: ComponentRadius,
        entity_id: Entity,
    ) -> PhysicsComponent {
        let capsule = ShapeHandle::new(ncollide2d::shape::Ball::new(radius.get()));
        let mut collider_desc = ColliderDesc::new(capsule)
            .collision_groups(CollisionGroups::new()
                .with_membership(&[LIVING_COLLISION_GROUP])
                .with_blacklist(&[])
            )
            .density(radius.0 as f32 * 5.0);
        let mut rb_desc = RigidBodyDesc::new()
            .user_data(entity_id)
            .collider(&collider_desc);
        let handle = rb_desc
            .gravity_enabled(false)
            .set_translation(pos)
            .build(world)
            .handle();
        PhysicsComponent {
            radius: radius,
            body_handle: handle,
        }
    }

    pub fn pos(&self, physics_world: &PhysicsWorld) -> Vector2<f32> {
        let body = physics_world.rigid_body(self.body_handle).unwrap();
        body.position().translation.vector
    }
}

#[derive(Clone)]
pub enum CharState {
    Idle,
    Walking(Point2<f32>),
    Sitting,
    PickingItem,
    StandBy,
    Attacking { attack_ends: ElapsedTime },
    ReceivingDamage,
    Freeze,
    Dead,
    CastingSkill {
        cast_started: ElapsedTime,
        cast_ends: ElapsedTime,
        can_move: bool,
        skill: Arc<Mutex<Box<SkillDescriptor>>>,
    },
}

unsafe impl Sync for CharState {}

unsafe impl Send for CharState {}

impl PartialEq  for CharState {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for CharState {}

impl CharState {
    pub fn is_attacking(&self) -> bool {
        match self {
            CharState::Attacking { attack_ends: _ } => true,
            _ => false
        }
    }

    pub fn is_casting(&self) -> bool {
        match self {
            CharState::CastingSkill { cast_started:_, cast_ends: _, can_move: _, skill: _ } => true,
            _ => false
        }
    }

    pub fn is_walking(&self) -> bool {
        match self {
            CharState::Walking(_pos) => true,
            _ => false
        }
    }

    pub fn get_sprite_index(&self) -> ActionIndex {
        match self {
            CharState::Idle => ActionIndex::Idle,
            CharState::Walking(_pos) => ActionIndex::Walking,
            CharState::Sitting => ActionIndex::Sitting,
            CharState::PickingItem => ActionIndex::PickingItem,
            CharState::StandBy => ActionIndex::StandBy,
            CharState::Attacking { attack_ends: _ } => ActionIndex::Attacking1,
            CharState::ReceivingDamage => ActionIndex::ReceivingDamage,
            CharState::Freeze => ActionIndex::Freeze1,
            CharState::Dead => ActionIndex::Dead,
            CharState::CastingSkill { cast_started:_, cast_ends: _, can_move: _, skill: _ } => ActionIndex::CastingSpell,
        }
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

#[derive(Debug)]
pub enum EntityTarget {
    OtherEntity(Entity),
    Pos(Point2<f32>),
}

#[derive(Component)]
pub struct CharacterStateComponent {
    pub target: Option<EntityTarget>,
    state: CharState,
    prev_state: CharState,
    pub moving_speed: f32,
    pub attack_range: f32,
    pub attack_speed: f32,
    // attack count per seconds
    pub bounding_rect_2d: SpriteBoundingRect,
    // attacks per second
    dir: usize,
    pub cannot_control_until: ElapsedTime,
}

impl CharacterStateComponent {
    pub fn new() -> CharacterStateComponent {
        CharacterStateComponent {
            target: None,
            moving_speed: 600.0,
            attack_range: 2.0,
            state: CharState::Idle,
            prev_state: CharState::Idle,
            attack_speed: 2.0,
            dir: 0,
            bounding_rect_2d: SpriteBoundingRect::default(),
            cannot_control_until: ElapsedTime(0.0),
        }
    }

    pub fn set_and_get_state_change(&mut self) -> bool {
        let ret = self.prev_state != self.state;
        // TODO: is it necessary to clone here?
        self.prev_state = self.state.clone();
        return ret;
    }

    pub fn can_move(&self, sys_time: &ElapsedTime) -> bool {
        let can_move_by_state = match self.state {
            CharState::CastingSkill { cast_started:_, cast_ends: _, can_move, skill: _ } => can_move,
            CharState::Idle => true,
            CharState::Walking(_pos) => true,
            CharState::Sitting => true,
            CharState::PickingItem => false,
            CharState::StandBy => true,
            CharState::Attacking { attack_ends: _ } => false,
            CharState::ReceivingDamage => true,
            CharState::Freeze => false,
            CharState::Dead => false,
        };
        can_move_by_state && self.cannot_control_until.has_passed(&sys_time)
    }

    pub fn state(&self) -> &CharState {
        &self.state
    }

    pub fn dir(&self) -> usize {
        self.dir
    }

    pub fn set_state(&mut self,
                     state: CharState,
                     dir: usize,
                     /*anim_sprite: &mut SpriteRenderDescriptor,
                     animation_started: ElapsedTime,
                     animation_duration: Option<ElapsedTime>*/) {
        self.state = state;
        self.dir = dir;
//        anim_sprite.direction = dir;
//        anim_sprite.animation_started = animation_started;
//        anim_sprite.forced_duration = animation_duration;
//        anim_sprite.action_index = state.get_sprite_index() as usize;
    }

    pub fn set_dir(&mut self, dir: usize) {
        self.dir = dir;
    }
}

#[derive(Component)]
pub struct PlayerSpriteComponent {
    pub job_id: JobId,
    pub head_index: usize,
    pub sex: Sex,
    pub descr: SpriteRenderDescriptor,
}

#[derive(Component)]
pub struct MonsterSpriteComponent {
    pub monster_id: MonsterId,
    pub descr: SpriteRenderDescriptor,
}

pub struct SpriteRenderDescriptor {
    pub action_index: usize,
    pub animation_started: ElapsedTime,
    pub forced_duration: Option<ElapsedTime>,
    pub direction: usize,
}