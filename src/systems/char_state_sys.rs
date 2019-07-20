use nalgebra::{Isometry2, Matrix4, Perspective3, Point2, Point3, Vector2, Vector3, Vector4};
use ncollide2d::query::point_internal::point_query::PointQuery;
use ncollide2d::shape::{Ball, Cuboid};
use nphysics2d::object::Body;
use rand::Rng;
use sdl2::keyboard::Scancode;
use specs::join::JoinIter;
use specs::prelude::*;
use specs::world::EntitiesRes;

use crate::{CharActionIndex, ElapsedTime, PhysicsWorld, RenderMatrices, Tick, TICKS_PER_SECOND};
use crate::cam::Camera;
use crate::components::{FlyingNumberComponent, FlyingNumberType, AttackComponent, AttackType};
use crate::components::char::{CharacterStateComponent, CharState, PhysicsComponent, PlayerSpriteComponent, EntityTarget, MonsterSpriteComponent};
use crate::components::skill::{PushBackWallSkill, SkillManifestationComponent};
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::systems::render::DIRECTION_TABLE;
use crate::video::{VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::systems::control_sys::CharacterControlSystem;
use crate::systems::atk_calc::{AttackCalculation, AttackOutcome};
use std::collections::HashMap;
use crate::components::controller::WorldCoords;

pub struct CharacterStateUpdateSystem;

impl<'a> specs::System<'a> for CharacterStateUpdateSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, PlayerSpriteComponent>,
        specs::WriteStorage<'a, MonsterSpriteComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(&mut self, (
        entities,
        mut physics_storage,
        mut char_state_storage,
        mut sprite_storage,
        mut monster_sprite_storage,
        mut system_vars,
        mut physics_world,
        mut system_benchmark,
        mut updater,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("CharacterStateUpdateSystem");

        // TODO: HACK
        // I can't get the position of the target entity inside the loop because
        // char_state storage is borrowed as mutable already
        let mut char_positions = HashMap::<Entity, WorldCoords>::new();
        for (char_entity_id, char_comp) in (&entities,
                                            &mut char_state_storage).join() {
            char_positions.insert(char_entity_id, char_comp.pos());
        }

        for (char_entity_id, char_comp) in (&entities, &mut char_state_storage).join() {
            // for autocompletion...
            let char_comp: &mut CharacterStateComponent = char_comp;

            if char_comp.hp <= 0 {
                char_comp.set_state(CharState::Dead, char_comp.dir());
                // remove rigid bodies from the physic simulation
                if let Some(phys_comp) = physics_storage.get(char_entity_id) {
                    let body_handle = phys_comp.body_handle;
                    physics_world.remove_bodies(&[body_handle]);
                    physics_storage.remove(char_entity_id);
                }
                continue;
            }

            let char_pos = char_comp.pos().coords;
            match char_comp.state().clone() {
                CharState::CastingSkill(casting_info) => {
                    if casting_info.cast_ends.has_passed(system_vars.time) {
                        let skill_entity_id = entities.create();

                        let manifestation = casting_info.skill.lock().unwrap().create_manifestation(
                            char_entity_id,
                            &char_pos,
                            &casting_info.mouse_pos_when_casted,
                            &mut physics_world,
                            &system_vars,
                            &entities,
                            &mut updater,
                        );
                        updater.insert(skill_entity_id, SkillManifestationComponent::new(
                            skill_entity_id,
                            manifestation),
                        );

                        char_comp.set_state(CharState::Idle, char_comp.dir());
                    }
                }
                CharState::Attacking { attack_ends, target } => {
                    if attack_ends.has_passed(system_vars.time) {
                        char_comp.set_state(CharState::Idle, char_comp.dir());

                        let damage_entity = entities.create();
                        updater.insert(damage_entity, AttackComponent {
                            src_entity: char_entity_id,
                            dst_entity: target,
                            typ: AttackType::Basic,
                        });
                    }
                }
                _ => {}
            }

            if char_comp.can_move(system_vars.time) {
                if let Some(target) = &char_comp.target {
                    if let EntityTarget::OtherEntity(target_entity) = target {
                        let target_pos = char_positions.get(target_entity);
                        if let Some(target_pos) = target_pos { // the target could have been removed
                            let distance = nalgebra::distance(&nalgebra::Point::from(char_pos), target_pos);
                            if distance <= char_comp.attack_range.multiply(2.0) {
                                let attack_anim_duration = ElapsedTime(1.0 / char_comp.attack_speed.as_f32());
                                let attack_ends = system_vars.time.add(attack_anim_duration);
                                let new_state = CharState::Attacking {
                                    attack_ends,
                                    target: *target_entity,
                                };
                                char_comp.set_state(new_state,
                                                    CharacterControlSystem::determine_dir(
                                                        target_pos,
                                                        &char_pos,
                                                    ));
                            } else {
                                // move closer
                                char_comp.set_state(
                                    CharState::Walking(*target_pos),
                                    CharacterControlSystem::determine_dir(target_pos, &char_pos),
                                );
                            }
                        } else {
                            char_comp.set_state(CharState::Idle, char_comp.dir());
                        }
                    } else if let EntityTarget::Pos(target_pos) = target {
                        let distance = nalgebra::distance(&nalgebra::Point::from(char_pos), &target_pos);
                        if distance <= 0.2 {
                            // stop
                            char_comp.set_state(CharState::Idle, char_comp.dir());
                        } else {
                            // move closer
                            char_comp.set_state(
                                CharState::Walking(*target_pos),
                                CharacterControlSystem::determine_dir(target_pos, &char_pos),
                            );
                        }
                    }
                } else { // no target and no receieving damage, casting or attacking
                    char_comp.set_state(CharState::Idle, char_comp.dir());
                }
            }
        }
        // apply moving physics here, so that the prev loop does not have to borrow physics_storage
        for (char_comp, physics_comp) in (&mut char_state_storage,
                                          &physics_storage).join() {
            if let CharState::Walking((target_pos)) = char_comp.state() {
                let dir = (target_pos - char_comp.pos()).normalize();
                let speed = dir * char_comp.moving_speed.multiply(600.0 * 0.01);
                let force = speed;
                let body = physics_world.rigid_body_mut(physics_comp.body_handle).unwrap();
                body.set_linear_velocity(body.velocity().linear + force);
            } else {
                let body = physics_world.rigid_body_mut(physics_comp.body_handle).unwrap();
                body.set_linear_velocity(Vector2::new(0.0, 0.0));
            }
        }
        // update animations based on current state
        for (char_comp, sprite) in (&mut char_state_storage, &mut sprite_storage).join() {
            if char_comp.set_and_get_state_change() {
                let state = char_comp.state();
                sprite.descr.animation_started = system_vars.time;
                sprite.descr.forced_duration = match state {
                    CharState::Attacking { attack_ends, .. } => Some(attack_ends.minus(system_vars.time)),
                    CharState::CastingSkill(casting_info) => Some(casting_info.cast_ends.minus(system_vars.time)),
                    _ => None
                };
                sprite.descr.action_index = state.get_sprite_index(false) as usize;
            }
            sprite.descr.direction = char_comp.dir();
        }
        for (char_comp, sprite) in (&mut char_state_storage, &mut monster_sprite_storage).join() {
            if char_comp.set_and_get_state_change() {
                let state = char_comp.state();
                sprite.descr.animation_started = system_vars.time;
                sprite.descr.forced_duration = match state {
                    CharState::Attacking { attack_ends, .. } => Some(attack_ends.minus(system_vars.time)),
                    CharState::CastingSkill(casting_info) => Some(casting_info.cast_ends.minus(system_vars.time)),
                    _ => None
                };
                sprite.descr.action_index = state.get_sprite_index(true);
            }
            sprite.descr.direction = char_comp.dir();
        }
    }
}