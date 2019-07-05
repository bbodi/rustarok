extern crate rand;

use crate::cam::Camera;
use websocket::stream::sync::TcpStream;
use std::sync::Mutex;
use nalgebra::{Point3, Vector3, Vector2, Point2};
use std::collections::HashSet;
use sdl2::keyboard::Scancode;
use crate::{Tick, LIVING_COLLISION_GROUP, STATIC_MODELS_COLLISION_GROUP, ActionIndex};
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
    pub char: Option<Entity>,
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
    pub fn new(x: f32, z: f32) -> ControllerComponent {
        let pitch = -60.0;
        let yaw = 270.0;
        let mut camera = Camera::new(Point3::new(x, 30.0, z));
        camera.rotate(pitch, yaw);
        ControllerComponent {
            char: None,
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


#[derive(Component)]
pub struct DummyAiComponent {
    pub target_pos: Point2<f32>,
    pub state: ActionIndex,
    pub controller: Option<Entity>,
    pub moving_speed: f32,
}

#[derive(Component)]
pub struct SimpleSpriteComponent {
    pub file_index: usize,
    pub action_index: usize,
    pub animation_start: Tick,
    pub direction: usize,
    pub is_monster: bool,
}

#[derive(Component)]
pub struct ExtraSpriteComponent {
    pub head_index: usize,
}

// radius = ComponentRadius * 0.5f32
#[derive(Eq, PartialEq, Hash)]
pub struct ComponentRadius(pub i32);

#[derive(Component)]
pub struct PhysicsComponent {
    pub radius: ComponentRadius,
    pub handle: nphysics2d::object::BodyHandle
}

impl PhysicsComponent {
    pub fn new(
        world: &mut nphysics2d::world::World<f32>,
        pos: Vector2<f32>,
    ) -> PhysicsComponent {
        let mut rng = rand::thread_rng();
        let radius = rng.gen_range(1, 5);
        let capsule = ShapeHandle::new(ncollide2d::shape::Ball::new(radius as f32 * 0.5));
        let mut collider_desc = ColliderDesc::new(capsule)
            .collision_groups(CollisionGroups::new()
                .with_membership(&[LIVING_COLLISION_GROUP])
                .with_blacklist(&[])
                .with_whitelist(&[STATIC_MODELS_COLLISION_GROUP, LIVING_COLLISION_GROUP]))
            .density(radius as f32 * 0.5);
        let mut rb_desc = RigidBodyDesc::new().collider(&collider_desc);
        let handle = rb_desc
            .set_translation(pos)
            .build(world).handle();
        PhysicsComponent {
            radius: ComponentRadius(radius),
            handle
        }
    }
}