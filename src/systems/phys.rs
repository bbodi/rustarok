use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::PhysicsWorld;

pub struct PhysicsSystem;

impl<'a> specs::System<'a> for PhysicsSystem {
    type SystemData = (
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        mut physics_world,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("PhysicsSystem");
        physics_world.step();
    }
}