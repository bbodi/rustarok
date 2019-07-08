use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::PhysicsWorld;
use nalgebra::Vector2;
use specs::prelude::*;
use crate::components::char::PhysicsComponent;

pub struct PhysicsSystem;
pub struct FrictionSystem;

impl<'a> specs::System<'a> for FrictionSystem {
    type SystemData = (
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(&mut self, (
        mut physics_world,
        mut system_benchmark,
        mut physics_storage,
        system_vars,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("FrictionSystem");
        for physics in (&physics_storage).join() {
            let body = physics_world.rigid_body_mut(physics.body_handle).unwrap();
            body.set_linear_velocity(Vector2::zeros());
        }
    }
}

impl<'a> specs::System<'a> for PhysicsSystem {
    type SystemData = (
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(&mut self, (
        mut physics_world,
        mut system_benchmark,
        system_vars,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("PhysicsSystem");
        physics_world.set_timestep(system_vars.dt.0);
        physics_world.step();
    }
}