use crate::cam::Camera;
use websocket::stream::sync::TcpStream;
use std::sync::Mutex;
use nalgebra::{Point3, Vector3, Vector2, Point2};
use std::collections::HashSet;
use sdl2::keyboard::Scancode;
use crate::{Tick, LIVING_COLLISION_GROUP, STATIC_MODELS_COLLISION_GROUP};
use specs::prelude::*;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{ColliderDesc, RigidBodyDesc};
use ncollide2d::world::CollisionGroups;

#[derive(Component)]
pub struct BrowserClient {
    pub websocket: Mutex<websocket::sync::Client<TcpStream>>,
    pub offscreen: Vec<u8>,
    pub ping: u16,
}

#[derive(Component)]
pub struct PositionComponent(pub Vector3<f32>);

#[derive(Component)]
pub struct ControllerComponent {
    pub camera: Camera,
    pub inputs: Vec<sdl2::event::Event>,
    pub keys: HashSet<Scancode>,
    pub mouse_down: bool,
    pub last_mouse_x: u16,
    pub last_mouse_y: u16,
    pub yaw: f32,
    pub pitch: f32,
}

impl ControllerComponent {
    pub fn new() -> ControllerComponent {
        ControllerComponent {
            camera: Camera::new(Point3::new(0.0, 0.0, 3.0)),
            inputs: vec![],
            keys: Default::default(),
            mouse_down: false,
            last_mouse_x: 400,
            last_mouse_y: 300,
            yaw: 270.0,
            pitch: 0.0,
        }
    }
}


#[derive(Component)]
pub struct DummyAiComponent {
    pub target_pos: Point2<f32>,
    pub state: i32,
    // 0 standing, 1 walking
    pub controller: Option<Entity>,
}

#[derive(Component)]
pub struct DirectionComponent(pub f32);

#[derive(Component)]
pub struct AnimatedSpriteComponent {
    pub file_index: usize,
    pub head_index: usize,
    pub action_index: usize,
    pub animation_start: Tick,
    pub direction: usize,
}

#[derive(Component, Clone)]
pub struct PhysicsComponent {
    pub handle: nphysics2d::object::BodyHandle
}

impl PhysicsComponent {
    pub fn new(
        world: &mut nphysics2d::world::World<f32>,
        pos: Vector2<f32>,
    ) -> PhysicsComponent {
        let capsule = ShapeHandle::new(ncollide2d::shape::Ball::new(1.0));
        let mut collider_desc = ColliderDesc::new(capsule)
            .collision_groups(CollisionGroups::new()
                .with_membership(&[LIVING_COLLISION_GROUP])
                .with_blacklist(&[])
                .with_whitelist(&[STATIC_MODELS_COLLISION_GROUP, LIVING_COLLISION_GROUP]))
            .density(1.3);
        let mut rb_desc = RigidBodyDesc::new().collider(&collider_desc);
        let handle = rb_desc
            .set_translation(pos)
            .build(world).handle();
        PhysicsComponent {
            handle
        }
    }
}