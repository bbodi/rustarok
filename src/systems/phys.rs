use crate::components::char::{CharacterStateComponent, PhysicsComponent};
use crate::systems::{Collision, CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::PhysicsWorld;
use nalgebra::Vector2;
use ncollide2d::events::ContactEvent;
use ncollide2d::query::Proximity;
use specs::prelude::*;

pub struct PhysCollisionCollectorSystem;

pub struct FrictionSystem;

impl<'a> specs::System<'a> for FrictionSystem {
    type SystemData = (
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (
        mut physics_world,
        mut system_benchmark,
        physics_storage,
        mut char_storage,
        system_vars,
    ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("FrictionSystem");
        for (physics, char_state) in (&physics_storage, &mut char_storage).join() {
            let body = physics_world.rigid_body_mut(physics.body_handle).unwrap();
            if char_state.cannot_control_until.has_passed(system_vars.time) {
                body.set_linear_velocity(Vector2::zeros());
            } else {
                let linear = body.velocity().linear;
                if linear.x != 0.0 || linear.y != 0.0 {
                    let dir = linear.normalize();
                    let slowing_vector = body.velocity().linear - (dir * 1.0);
                    let len = slowing_vector.magnitude();
                    if len <= 0.0001 {
                        body.set_linear_velocity(Vector2::zeros());
                    } else {
                        body.set_linear_velocity(slowing_vector);
                    }
                }
            }
            let body_pos = body.position().translation.vector;
            char_state.set_pos_dont_use_it(body_pos);
        }
    }
}

impl<'a> specs::System<'a> for PhysCollisionCollectorSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, CollisionsFromPrevFrame>,
    );

    fn run(
        &mut self,
        (
            _entities,
            mut physics_world,
            mut system_benchmark,
            system_vars,
            mut collisions_resource,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("PhysicsSystem");

        physics_world.set_timestep(system_vars.dt.0);
        physics_world.step();

        for event in physics_world.proximity_events() {
            log::trace!("{:?}", event);
            let collider1 = physics_world.collider(event.collider1).unwrap();
            let collider1_body_handle = collider1.body();
            let collider2 = physics_world.collider(event.collider2).unwrap();
            let (character_coll_handle, other_coll_handle) = if collider1_body_handle.is_ground() {
                (collider2.handle(), collider1.handle())
            } else {
                (collider1.handle(), collider2.handle())
            };
            let collision = Collision {
                character_coll_handle,
                other_coll_handle,
            };
            match event.new_status {
                Proximity::Intersecting => {
                    collisions_resource
                        .collisions
                        .insert((collider1.handle(), collider2.handle()), collision);
                    dbg!(&collisions_resource.collisions);
                }
                Proximity::WithinMargin => {
                    if event.prev_status == Proximity::Intersecting {
                        collisions_resource
                            .collisions
                            .remove(&(collider1.handle(), collider2.handle()));
                        dbg!(&collisions_resource.collisions);
                    }
                }
                Proximity::Disjoint => {
                    collisions_resource
                        .collisions
                        .remove(&(collider1.handle(), collider2.handle()));
                    dbg!(&collisions_resource.collisions);
                }
            }
        }

        for event in physics_world.contact_events() {
            log::trace!("{:?}", event);
            match event {
                ContactEvent::Started(handle1, handle2) => {
                    let collider1 = physics_world.collider(*handle1).unwrap();
                    let collider1_body = collider1.body();
                    let collider2 = physics_world.collider(*handle2).unwrap();
                    let (character_coll_handle, other_coll_handle) = if collider1_body.is_ground() {
                        (collider2.handle(), collider1.handle())
                    } else {
                        (collider1.handle(), collider2.handle())
                    };
                    let collision = Collision {
                        character_coll_handle,
                        other_coll_handle,
                    };
                    collisions_resource
                        .collisions
                        .insert((collider1.handle(), collider2.handle()), collision);
                }
                ContactEvent::Stopped(handle1, handle2) => {
                    let collider1 = physics_world.collider(*handle1).unwrap();
                    let collider1_body = collider1.body();
                    let collider2 = physics_world.collider(*handle2).unwrap();
                    let (character_coll_handle, other_coll_handle) = if collider1_body.is_ground() {
                        (collider2.handle(), collider1.handle())
                    } else {
                        (collider1.handle(), collider2.handle())
                    };
                    let _collision = Collision {
                        character_coll_handle,
                        other_coll_handle,
                    };
                    collisions_resource
                        .collisions
                        .remove(&(collider1.handle(), collider2.handle()));
                }
            }
        }
    }
}
