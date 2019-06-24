use crate::cam::Camera;
use websocket::stream::sync::TcpStream;
use std::sync::Mutex;
use nalgebra::{Point3, Vector3};
use ncollide3d::shape::ShapeHandle;
use nphysics3d::object::{ColliderDesc, RigidBodyDesc};
use std::collections::HashSet;
use sdl2::keyboard::Scancode;
use crate::Tick;
use specs::prelude::*;

#[derive(Component)]
pub struct CameraComponent {
    pub camera: Camera,
    pub mouse_down: bool,
    pub last_mouse_x: u16,
    pub last_mouse_y: u16,
    pub yaw: f32,
    pub pitch: f32,
}

impl CameraComponent {
    pub fn new() -> CameraComponent {
        CameraComponent {
            camera: Camera::new(Point3::new(0.0, 0.0, 3.0)),
            mouse_down: false,
            last_mouse_x: 400,
            last_mouse_y: 300,
            yaw: 270.0,
            pitch: 0.0,
        }
    }
}

#[derive(Component)]
pub struct BrowserClient {
    pub websocket: Mutex<websocket::sync::Client<TcpStream>>,
    pub offscreen: Vec<u8>,
    pub ping: u16,
}

#[derive(Component)]
pub struct PositionComponent(pub Vector3<f32>);

#[derive(Component, Default)]
pub struct InputProducerComponent {
    pub inputs: Vec<sdl2::event::Event>,
    pub keys: HashSet<Scancode>,
}


#[derive(Component)]
pub struct DummyAiComponent {
    pub target_pos: Point3<f32>,
    pub state: i32, // 0 standing, 1 walking
}

#[derive(Component)]
pub struct DirectionComponent(pub f32);

#[derive(Component)]
pub struct AnimatedSpriteComponent {
    pub file_index: usize,
    pub action_index: usize,
    pub animation_start: Tick,
    pub direction: usize,
}

#[derive(Component, Clone)]
pub struct PhysicsComponent {
    pub handle: nphysics3d::object::BodyHandle
}

impl PhysicsComponent {
    pub fn new(world: &mut nphysics3d::world::World<f32>,
           pos: Vector3<f32>) -> PhysicsComponent {
        let capsule = ShapeHandle::new(ncollide3d::shape::Capsule::new(2.0, 1.0));
        let mut collider_desc = ColliderDesc::new(capsule)
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