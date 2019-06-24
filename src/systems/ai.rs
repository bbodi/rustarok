use crate::components::{PositionComponent, PhysicsComponent, DummyAiComponent, AnimatedSpriteComponent};
use nalgebra::Point3;
use crate::systems::render::DIRECTION_TABLE;
use specs::prelude::*;
use rand::Rng;
use crate::systems::{SystemVariables, SystemFrameDurations};

pub struct DummyAiSystem;
impl<'a> specs::System<'a> for DummyAiSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, PositionComponent>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, DummyAiComponent>,
        specs::WriteStorage<'a, AnimatedSpriteComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        entities,
        mut position_storage,
        mut physics_storage,
        mut ai_storage,
        mut animated_sprite_storage,
        mut system_vars,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("DummyAiSystem");
        let mut rng = rand::thread_rng();
        for (entity, pos, ai, physics_comp) in (&entities, &mut position_storage, &mut ai_storage, &mut physics_storage).join() {
            let mut body = system_vars.physics_world.rigid_body_mut(physics_comp.handle).unwrap();
            let pos = body.position().translation.vector;
            if nalgebra::distance(&nalgebra::Point::from(pos), &ai.target_pos) < 10.0 {
                ai.target_pos = Point3::<f32>::new(0.5 * 200.0 * (rng.gen::<f32>()), 0.5, -(0.5 * 200.0 * (rng.gen::<f32>())));
                if let Some(anim_sprite) = animated_sprite_storage.get_mut(entity) {
                    let dir_vec = ai.target_pos - pos;
                    // "- 90.0"
                    // The calculated yaw for the camera are 90 at [0;1] and 180 at [1;0] etc,
                    // this calculation gives a different result which is shifted 90 degrees clockwise,
                    // so it is 90 at [1;0].
                    let dd = dir_vec.x.atan2(dir_vec.z).to_degrees() - 90.0;
                    let dd = if dd < 0.0 { dd + 360.0 } else if dd > 360.0 { dd - 360.0 } else { dd };
                    let dir_index = (dd / 45.0 + 0.5) as usize % 8;
                    anim_sprite.direction = DIRECTION_TABLE[dir_index];
                }
            } else {
                let mut force = (ai.target_pos - nalgebra::Point::from(pos)).normalize() * 10.0;
                body.set_linear_velocity(force);
            }
        }
    }
}
