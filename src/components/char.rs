use ncollide2d::shape::ShapeHandle;
use nalgebra::{Vector2, Point2};
use nphysics2d::object::{ColliderDesc, RigidBodyDesc};
use ncollide2d::world::CollisionGroups;
use crate::{LIVING_COLLISION_GROUP, STATIC_MODELS_COLLISION_GROUP, PhysicsWorld, Tick, ActionIndex, ElapsedTime};
use specs::Entity;
use specs::prelude::*;
use crate::consts::{MonsterId, JobId};
use crate::systems::Sex;
use crate::components::skill::SkillDescriptor;
use crate::systems::control_sys::CharacterControlSystem;
use std::sync::{Mutex, Arc};
use std::ops::Mul;
use crate::components::controller::WorldCoords;

pub fn create_char(
    ecs_world: &mut specs::world::World,
    pos2d: Point2<f32>,
    sex: Sex,
    job_id: JobId,
    head_index: usize,
    radius: i32,
) -> Entity {
    let entity_id = {
        let mut char_comp = CharacterStateComponent::new();
        char_comp.armor = U8Float::new(Percentage::new(10.0));
        let mut entity_builder = ecs_world.create_entity()
            .with(char_comp);
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
pub struct CastingSkillData {
    pub mouse_pos_when_casted: WorldCoords,
    pub cast_started: ElapsedTime,
    pub cast_ends: ElapsedTime,
    pub can_move: bool,
    pub skill: Arc<Mutex<Box<SkillDescriptor>>>,
}

#[derive(Clone)]
pub enum CharState {
    Idle,
    Walking(Point2<f32>),
    Sitting,
    PickingItem,
    StandBy,
    Attacking { attack_ends: ElapsedTime, target: Entity },
    ReceivingDamage,
    Freeze,
    Dead,
    CastingSkill(CastingSkillData),
}

unsafe impl Sync for CharState {}

unsafe impl Send for CharState {}

impl PartialEq for CharState {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for CharState {}

impl CharState {
    pub fn is_attacking(&self) -> bool {
        match self {
            CharState::Attacking { attack_ends: _, target: _ } => true,
            _ => false
        }
    }

    pub fn is_casting(&self) -> bool {
        match self {
            CharState::CastingSkill { .. } => true,
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
            CharState::Attacking { attack_ends: _, target: _ } => ActionIndex::Attacking1,
            CharState::ReceivingDamage => ActionIndex::ReceivingDamage,
            CharState::Freeze => ActionIndex::Freeze1,
            CharState::Dead => ActionIndex::Dead,
            CharState::CastingSkill { .. } => ActionIndex::CastingSpell,
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

#[derive(Copy, Clone)]
pub struct Percentage(f32);

impl Percentage {
    pub fn new(percentage: f32) -> Percentage {
        Percentage(percentage / 100.0)
    }

    pub fn from_f32(percentage: f32) -> Percentage {
        Percentage(percentage)
    }

    pub fn as_f32(&self) -> f32 {
        self.0
    }
}

/// Representing f32 values from 0 to 1.0 or 0% to 100%, with 0.1% increments
/// e.g. U16Float(550).as_f32() == 55% == 0.55
/// e.g. U16Float(10).as_f32() == 1% == 0.01
/// e.g. U16Float(12).as_f32() == 1.2% == 0.012
/// e.g. U16Float(60000).as_f32() == 600% == 60.0
#[derive(Copy, Clone, Debug)]
pub struct U16Float(u16);

impl U16Float {
    pub fn new(p: Percentage) -> U16Float {
        U16Float((p.as_f32() * 1000.0) as u16)
    }

    pub fn as_f32(&self) -> f32 {
        self.0 as f32 / 1000.0
    }
}

/// Representing f32 values from 0 to 5.1 or 0% to 510%, with 0.1% increments
/// e.g. U8Float(250).as_f32() == 500% == 5
/// e.g. U8Float(10).as_f32() == 20% == 0.2
/// e.g. U8Float(12).as_f32() == 24% == 0.24
/// e.g. U8Float(1).as_f32() == 2% == 0.02
#[derive(Copy, Clone, Debug)]
pub struct U8Float(u8);

impl U8Float {
    pub fn new(p: Percentage) -> U8Float {
        U8Float((p.as_f32() * 50.0) as u8)
    }

    pub fn as_f32(&self) -> f32 {
        self.0 as f32 / 50.0
    }

    pub fn multiply(&self, num: f32) -> f32 {
        self.as_f32() * num
    }

    pub fn add_me_to_as_percentage(&self, num: f32) -> f32 {
        num + (self.as_f32() * num)
    }

    pub fn subtract_me_from_as_percentage(&self, num: f32) -> f32 {
        num - (self.as_f32() * num)
    }
}

#[derive(Component)]
pub struct CharacterStateComponent {
    pub target: Option<EntityTarget>,
    state: CharState,
    prev_state: CharState,
    // attack count per seconds
    pub bounding_rect_2d: SpriteBoundingRect,
    // attacks per second
    dir: usize,
    pub cannot_control_until: ElapsedTime,

    pub max_hp: u32,
    pub hp: i32,
    pub moving_speed: U8Float,
    pub attack_range: U8Float,
    pub attack_speed: U8Float,
    pub attack_damage: u16,
    pub attack_damage_bonus: U8Float,
    pub armor: U8Float,
}

impl CharacterStateComponent {
    pub fn new() -> CharacterStateComponent {
        CharacterStateComponent {
            target: None,
            moving_speed: U8Float::new(Percentage::new(100.0)),
            attack_range: U8Float::new(Percentage::new(100.0)),
            state: CharState::Idle,
            prev_state: CharState::Idle,
            attack_speed: U8Float::new(Percentage::new(100.0)),
            attack_damage: 76,
            attack_damage_bonus: U8Float::new(Percentage::new(0.0)),
            armor: U8Float::new(Percentage::new(0.0)),
            dir: 0,
            bounding_rect_2d: SpriteBoundingRect::default(),
            cannot_control_until: ElapsedTime(0.0),
            max_hp: 2000,
            hp: 2000,
        }
    }

    pub fn set_and_get_state_change(&mut self) -> bool {
        let ret = self.prev_state != self.state;
        // TODO: is it necessary to clone here?
        self.prev_state = self.state.clone();
        return ret;
    }

    pub fn can_move(&self, sys_time: ElapsedTime) -> bool {
        let can_move_by_state = match &self.state {
            CharState::CastingSkill(casting_info) => casting_info.can_move,
            CharState::Idle => true,
            CharState::Walking(_pos) => true,
            CharState::Sitting => true,
            CharState::PickingItem => false,
            CharState::StandBy => true,
            CharState::Attacking { attack_ends: _, target: _ } => false,
            CharState::ReceivingDamage => true,
            CharState::Freeze => false,
            CharState::Dead => false,
        };
        can_move_by_state && self.cannot_control_until.has_passed(sys_time)
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