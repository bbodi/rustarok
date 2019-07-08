use specs::Entity;
use crate::cam::Camera;
use std::collections::HashSet;
use sdl2::keyboard::Scancode;
use nalgebra::Point3;
use specs::prelude::*;

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
