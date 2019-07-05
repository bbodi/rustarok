use crate::components::{PhysicsComponent, DummyAiComponent, PlayerSpriteComponent, ControllerComponent, FlyingNumberComponent};
use nalgebra::{Point3, Point2, Vector2, Vector3, Perspective3, Vector4};
use crate::systems::render::DIRECTION_TABLE;
use specs::prelude::*;
use rand::Rng;
use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::video::{VIDEO_WIDTH, VIDEO_HEIGHT};
use sdl2::keyboard::Scancode;
use crate::ActionIndex;

pub struct DummyAiSystem;

impl<'a> specs::System<'a> for DummyAiSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, DummyAiComponent>,
        specs::WriteStorage<'a, PlayerSpriteComponent>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        entities,
        mut physics_storage,
        mut ai_storage,
        mut animated_sprite_storage,
        controller_storage,
        mut system_vars,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("DummyAiSystem");
        let mut rng = rand::thread_rng();
        let dt = system_vars.dt.0;
        for (entity, ai, physics_comp) in (&entities,
                                                &mut ai_storage,
                                                &mut physics_storage).join() {
            let projection_matrix = system_vars.matrices.projection.clone();
            let view_matrix = system_vars.matrices.view.clone();
            let body = system_vars.map_render_data.physics_world.rigid_body_mut(physics_comp.handle).unwrap();
            let pos = body.position().translation.vector;
            if let Some(controller_entity) = ai.controller {
                let controller = controller_storage.get(controller_entity).unwrap();
                if controller.right_mouse_released {
                    let screen_point = Point2::new(controller.last_mouse_x as f32, controller.last_mouse_y as f32);

                    let ray_clip = Vector4::new(2.0 * screen_point.x as f32 / VIDEO_WIDTH as f32 - 1.0,
                                                1.0 - (2.0 * screen_point.y as f32) / VIDEO_HEIGHT as f32,
                                                -1.0,
                                                1.0);
                    let ray_eye = projection_matrix.try_inverse().unwrap() * ray_clip;
                    let ray_eye = Vector4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);
                    let ray_world = view_matrix.try_inverse().unwrap() * ray_eye;
                    let ray_world = Vector3::new(ray_world.x, ray_world.y, ray_world.z).normalize();

                    let line_location = controller.camera.pos();
                    let line_direction: Vector3<f32> = ray_world;
                    let plane_normal = Vector3::new(0.0, 1.0, 0.0);
                    let plane_point = Vector3::new(0.0, 0.0, 0.0);
                    let t = (plane_normal.dot(&plane_point) - plane_normal.dot(&line_location.coords)) / plane_normal.dot(&line_direction);
                    let world_pos = line_location + (line_direction.scale(t));
                    ai.target_pos = dbg!(Point2::new(world_pos.x, world_pos.z));
                    ai.state = ActionIndex::Walking;
                    if let Some(anim_sprite) = animated_sprite_storage.get_mut(entity) {
                        anim_sprite.base.direction = DummyAiSystem::determine_dir(&ai.target_pos, &pos);
                        anim_sprite.base.action_index = ai.state as usize;
                    }
                }
            }
            let distance = nalgebra::distance(&nalgebra::Point::from(pos), &ai.target_pos);
            if distance < 0.2 {
                if ai.controller.is_none() {
                    ai.target_pos = Point2::<f32>::new(0.5 * 200.0 * (rng.gen::<f32>()), -(0.5 * 200.0 * (rng.gen::<f32>())));
                }
                ai.state = ActionIndex::Idle;
                if let Some(anim_sprite) = animated_sprite_storage.get_mut(entity) {
                    anim_sprite.base.direction = DummyAiSystem::determine_dir(&ai.target_pos, &pos);
                    anim_sprite.base.action_index = ai.state as usize;
                }
                body.set_linear_velocity(Vector2::new(0.0, 0.0));
            } else {
                let dir = (ai.target_pos - nalgebra::Point::from(pos)).normalize();
                let speed = dir * ai.moving_speed * dt;
                let force = speed;
                body.set_linear_velocity(force);
            }
        }
    }
}

impl DummyAiSystem {
    fn determine_dir(&target_pos: &Point2<f32>, pos: &Vector2<f32>) -> usize {
        let dir_vec = target_pos - pos;
        // "- 90.0"
        // The calculated yaw for the camera are 90 at [0;1] and 180 at [1;0] etc,
        // this calculation gives a different result which is shifted 90 degrees clockwise,
        // so it is 90 at [1;0].
        let dd = dir_vec.x.atan2(dir_vec.y).to_degrees() - 90.0;
        let dd = if dd < 0.0 { dd + 360.0 } else if dd > 360.0 { dd - 360.0 } else { dd };
        let dir_index = (dd / 45.0 + 0.5) as usize % 8;
        return DIRECTION_TABLE[dir_index];
    }
}
