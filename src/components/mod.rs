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

#[derive(Component)]
pub struct BrowserClient {
    pub websocket: Mutex<websocket::sync::Client<TcpStream>>,
    pub offscreen: Vec<u8>,
    pub ping: u16,
}

#[derive(Component)]
pub struct ControllerComponent {
    pub char: Entity,
    pub camera: Camera,
    pub inputs: Vec<sdl2::event::Event>,
    pub keys: HashSet<Scancode>,
    pub left_mouse_down: bool,
    pub right_mouse_down: bool,
    pub left_mouse_released: bool,
    pub right_mouse_released: bool,
    pub last_mouse_x: u16,
    pub last_mouse_y: u16,
    pub yaw: f32,
    pub pitch: f32,
}

impl ControllerComponent {
    pub fn new(char: Entity, x: f32, z: f32) -> ControllerComponent {
        let pitch = -60.0;
        let yaw = 270.0;
        let mut camera = Camera::new(Point3::new(x, 20.0, z));
        camera.rotate(pitch, yaw);
        ControllerComponent {
            char,
            camera,
            inputs: vec![],
            keys: Default::default(),
            left_mouse_down: false,
            right_mouse_down: false,
            left_mouse_released: false,
            right_mouse_released: false,
            last_mouse_x: 400,
            last_mouse_y: 300,
            yaw,
            pitch,
        }
    }
}


//#[derive(Component)]
//pub struct CharacterStateComponent {
//    pub target_pos: Point2<f32>,
//    pub state: ActionIndex,
//    pub controller: Option<Entity>,
//}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharState {
    Idle,
    Walking,
    Sitting,
    PickingItem,
    StandBy,
    Attacking { attack_ends: Tick },
    ReceivingDamage,
    Freeze,
    Dead,
    CastingSpell,
}

impl CharState {
    pub fn is_attacking(&self) -> bool {
        match self {
            CharState::Attacking {attack_ends: _} => true,
            _ => false
        }
    }

    pub fn get_sprite_index(&self) -> ActionIndex {
        match self {
            CharState::Idle => ActionIndex::Idle,
            CharState::Walking => ActionIndex::Walking,
            CharState::Sitting => ActionIndex::Sitting,
            CharState::PickingItem => ActionIndex::PickingItem,
            CharState::StandBy => ActionIndex::StandBy,
            CharState::Attacking{attack_ends: _} => ActionIndex::Attacking1,
            CharState::ReceivingDamage => ActionIndex::ReceivingDamage,
            CharState::Freeze => ActionIndex::Freeze1,
            CharState::Dead => ActionIndex::Dead,
            CharState::CastingSpell => ActionIndex::CastingSpell,
        }
    }
}

#[derive(Component, Debug)]
pub struct CharacterStateComponent {
    pub target_pos: Option<Point2<f32>>,
    pub target: Option<Entity>,
    state: CharState,
    pub moving_speed: f32,
    pub attack_range: f32,
    pub attack_speed: f32,
    // attacks per second
    dir: usize,
}

impl CharacterStateComponent {
    pub fn new() -> CharacterStateComponent {
        CharacterStateComponent {
            target_pos: None,
            moving_speed: 600.0,
            attack_range: 2.0,
            state: CharState::Idle,
            target: None,
            attack_speed: 2.0,
            dir: 0,
        }
    }

    pub fn state(&self) -> CharState {
        self.state
    }

    pub fn dir(&self) -> usize {
        self.dir
    }

    pub fn set_state(&mut self, state: CharState, dir: usize, anim_sprite: &mut PlayerSpriteComponent, tick: Tick, finish: Option<Tick>) {
        self.state = state;
        self.dir = dir;
        anim_sprite.base.direction = dir;
        anim_sprite.base.animation_started = tick;
        anim_sprite.base.animation_finish = finish;
        anim_sprite.base.action_index = state.get_sprite_index() as usize;
    }

    pub fn set_dir(&mut self, dir: usize, anim_sprite: &mut PlayerSpriteComponent) {
        self.dir = dir;
        anim_sprite.base.direction = dir;
    }
}

#[derive(Component)]
pub struct PlayerSpriteComponent {
    pub base: MonsterSpriteComponent,
    pub head_index: usize,
}

#[derive(Component)]
pub struct MonsterSpriteComponent {
    pub file_index: usize,
    pub action_index: usize,
    pub animation_started: Tick,
    pub animation_finish: Option<Tick>,
    pub direction: usize,
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
    ) -> PhysicsComponent {
        let capsule = ShapeHandle::new(ncollide2d::shape::Ball::new(radius.get()));
        let mut collider_desc = ColliderDesc::new(capsule)
            .collision_groups(CollisionGroups::new()
                .with_membership(&[LIVING_COLLISION_GROUP])
                .with_blacklist(&[])
                .with_whitelist(&[STATIC_MODELS_COLLISION_GROUP, LIVING_COLLISION_GROUP]))
            .density(radius.0 as f32 * 5.0);
        let mut rb_desc = RigidBodyDesc::new().collider(&collider_desc);
        let handle = rb_desc
            .set_translation(pos)
            .build(world).handle();
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