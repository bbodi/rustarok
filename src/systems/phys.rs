use crate::components::char::{CharacterStateComponent, PhysicsComponent};
use crate::systems::{Collision, CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::PhysicsWorld;
use nalgebra::Vector2;
use ncollide2d::events::ContactEvent;
use ncollide2d::query::Proximity;
use specs::prelude::*;

pub struct PhysicsSystem;

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
        let stopwatch = system_benchmark.start_measurement("FrictionSystem");
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

impl<'a> specs::System<'a> for PhysicsSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, CollisionsFromPrevFrame>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut physics_world,
            mut system_benchmark,
            char_storage,
            physics_storage,
            system_vars,
            mut collisions_resource,
            updater,
        ): Self::SystemData,
    ) {
        let stopwatch = system_benchmark.start_measurement("PhysicsSystem");

        physics_world.set_timestep(system_vars.dt.0);
        physics_world.step();

        collisions_resource.collisions.clear();

        collisions_resource.collisions.extend(
            physics_world
                .proximity_events()
                .iter()
                .map(|event| {
                    log::trace!("{:?}", event);
                    if event.new_status == Proximity::Intersecting {
                        let collision = {
                            let collider1 = physics_world.collider(event.collider1).unwrap();
                            let collider1_body = collider1.body();
                            let collider2 = physics_world.collider(event.collider2).unwrap();
                            let collider2_body = collider2.body();
                            if collider1_body.is_ground() {
                                Collision {
                                    character_coll_handle: collider2.handle(),
                                    other_coll_handle: collider1.handle(),
                                }
                            } else {
                                Collision {
                                    character_coll_handle: collider1.handle(),
                                    other_coll_handle: collider2.handle(),
                                }
                            }
                        };
                        Some(collision)
                    } else {
                        None
                    }
                })
                .filter(|it| it.is_some())
                .map(|it| it.unwrap()),
        );
        collisions_resource.collisions.extend(
            physics_world
                .contact_events()
                .iter()
                .map(|event| {
                    log::trace!("{:?}", event);
                    if let ContactEvent::Started(handle1, handle2) = event {
                        let collision = {
                            let collider1 = physics_world.collider(*handle1).unwrap();
                            let collider1_body = collider1.body();
                            let collider2 = physics_world.collider(*handle2).unwrap();
                            let collider2_body = collider2.body();
                            if collider1_body.is_ground() {
                                Collision {
                                    character_coll_handle: collider2.handle(),
                                    other_coll_handle: collider1.handle(),
                                }
                            } else {
                                Collision {
                                    character_coll_handle: collider1.handle(),
                                    other_coll_handle: collider2.handle(),
                                }
                            }
                        };
                        Some(collision)
                    } else {
                        None
                    }
                })
                .filter(|it| it.is_some())
                .map(|it| it.unwrap()),
        );
    }
}
