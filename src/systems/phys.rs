use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::{PhysicsWorld, ElapsedTime};
use nalgebra::Vector2;
use specs::prelude::*;
use crate::components::char::{PhysicsComponent, CharacterStateComponent};
use ncollide2d::query::Proximity;
use crate::components::skill::PushBackWallSkill;
use nphysics2d::object::{Body, BodyHandle};

pub struct PhysicsSystem;

pub struct FrictionSystem;

impl<'a> specs::System<'a> for FrictionSystem {
    type SystemData = (
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(&mut self, (
        mut physics_world,
        mut system_benchmark,
        mut physics_storage,
        char_storage,
        system_vars,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("FrictionSystem");
        for (physics, char_state) in (&physics_storage, &char_storage).join() {
            let body = physics_world.rigid_body_mut(physics.body_handle).unwrap();
            if char_state.cannot_control_until.has_passed(&system_vars.time) {
                body.set_linear_velocity(Vector2::zeros());
            } else {
                let slowing_vector = body.velocity().linear - (body.velocity().linear * 0.1);
                body.set_linear_velocity(slowing_vector);
            }
        }
    }
}

impl<'a> specs::System<'a> for PhysicsSystem {
    type SystemData = (
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(&mut self, (
        mut physics_world,
        mut system_benchmark,
        mut char_storage,
        physics_storage,
        system_vars,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("PhysicsSystem");

        physics_world.set_timestep(system_vars.dt.0);
        physics_world.step();

        //
        let events = physics_world.proximity_events();
        let bodies: Vec<BodyHandle> = events.iter().map(|event| {
            if event.new_status == Proximity::Intersecting {
                let body_handle = {
                    let collider1_body = physics_world.collider(event.collider1).unwrap().body();
                    if collider1_body.is_ground() {
                        physics_world.collider(event.collider2).unwrap().body()
                    } else {
                        collider1_body
                    }
                };
                Some(body_handle)
            } else { None }
        }).filter(|it| it.is_some()).map(|it| it.unwrap()).collect();
        for body_handle in bodies {
            let body = physics_world.rigid_body_mut(body_handle).unwrap();
            let entity_id = body.user_data().map(|v| v.downcast_ref().unwrap()).unwrap();
            let char_state = char_storage.get_mut(*entity_id).unwrap();
            char_state.cannot_control_until.run_at_least_until_seconds(&system_vars.time, 1);
            body.set_linear_velocity(body.velocity().linear * -1.0);
        }
    }
}