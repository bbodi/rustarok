use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::{PhysicsWorld, ElapsedTime, SKILL_AREA_COLLISION_GROUP};
use nalgebra::Vector2;
use specs::prelude::*;
use crate::components::char::{PhysicsComponent, CharacterStateComponent, CharState};
use ncollide2d::query::Proximity;
use crate::components::skill::PushBackWallSkill;
use nphysics2d::object::{Body, BodyHandle, ColliderHandle, Collider};
use ncollide2d::events::{ContactEvent, ContactEvents};

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
        let mut bodies: Vec<(ColliderHandle, ColliderHandle)> = physics_world.proximity_events().iter().map(|event| {
            dbg!(&event);
            if event.new_status == Proximity::Intersecting {
                let (char_body_collider, other_obj_collider) = {
                    let collider1 = physics_world.collider(event.collider1).unwrap();
                    let collider1_body = collider1.body();
                    let collider2 = physics_world.collider(event.collider2).unwrap();
                    let collider2_body = collider2.body();
                    if collider1_body.is_ground() {
                        (collider2, collider1)
                    } else {
                        (collider1, collider2)
                    }
                };
                Some((char_body_collider.handle(), other_obj_collider.handle()))
            } else { None }
        }).filter(|it| it.is_some()).map(|it| it.unwrap()).collect();
        bodies.extend(
            physics_world.contact_events().iter().map(|event| {
                dbg!(&event);
                if let ContactEvent::Started(handle1, handle2) = event {
                    let (char_body_collider, other_obj_collider) = {
                        let collider1 = physics_world.collider(*handle1).unwrap();
                        let collider1_body = collider1.body();
                        let collider2 = physics_world.collider(*handle2).unwrap();
                        let collider2_body = collider2.body();
                        if collider1_body.is_ground() {
                            (collider2, collider1)
                        } else {
                            (collider1, collider2)
                        }
                    };
                    Some((char_body_collider.handle(), other_obj_collider.handle()))
                } else { None }
            }).filter(|it| it.is_some()).map(|it| it.unwrap())
        );

        for (char_collider_handle, other_obj_collider_handle) in bodies {
            let other_obj_collider = physics_world.collider(other_obj_collider_handle).unwrap();
            let collide_with_skill = other_obj_collider.collision_groups().is_member_of(SKILL_AREA_COLLISION_GROUP);
            if collide_with_skill {
                let char_body = physics_world.collider(char_collider_handle).unwrap().body();
                let body = physics_world.rigid_body_mut(char_body).unwrap();
                let entity_id = body.user_data().map(|v| v.downcast_ref().unwrap()).unwrap();
                let char_state = char_storage.get_mut(*entity_id).unwrap();
                char_state.cannot_control_until.run_at_least_until_seconds(&system_vars.time, 1);
                char_state.set_state(CharState::ReceivingDamage, char_state.dir());
                body.set_linear_velocity(body.velocity().linear * -1.0);
            }
        }
    }
}