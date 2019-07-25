use specs::prelude::*;

use crate::{ElapsedTime, PhysicsWorld};
use crate::components::{AttackComponent, AttackType};
use crate::components::char::{CharacterStateComponent, CharState, PhysicsComponent, EntityTarget, SpriteRenderDescriptorComponent};
use crate::components::skill::SkillManifestationComponent;
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::systems::control_sys::CharacterControlSystem;
use std::collections::HashMap;
use crate::components::controller::WorldCoords;
use nalgebra::Vector2;
use crate::components::skill::SkillDescriptor;

pub struct CharacterStateUpdateSystem;

impl<'a> specs::System<'a> for CharacterStateUpdateSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, SpriteRenderDescriptorComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(&mut self, (
        entities,
        mut physics_storage,
        mut char_state_storage,
        mut sprite_storage,
        system_vars,
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

            if char_comp.hp <= 0 && *char_comp.state() != CharState::Dead {
                log::debug!("Entity has died {:?}", char_entity_id);
                char_comp.set_state(CharState::Dead, char_comp.dir());
                // remove rigid bodies from the physic simulation
                if let Some(phys_comp) = physics_storage.get(char_entity_id) {
                    let body_handle = phys_comp.body_handle;
                    physics_world.remove_bodies(&[body_handle]);
                    physics_storage.remove(char_entity_id);
                }
                continue;
            }
            if *char_comp.state() == CharState::Dead {
                continue;
            }
//            char_comp
//                .statuses
//                .update(system_vars.time, &mut char_comp);

            let char_pos = char_comp.pos().coords;
            match char_comp.state().clone() {
                CharState::CastingSkill(casting_info) => {
                    if casting_info.cast_ends.has_passed(system_vars.time) {
                        log::debug!("Skill cast has finished: {:?}", casting_info.skill);
                        let manifestation = casting_info.skill.finish_cast(
                            char_entity_id,
                            &char_pos,
                            &casting_info.mouse_pos_when_casted,
                            casting_info.target_entity,
                            &mut physics_world,
                            &system_vars,
                            &entities,
                            &mut updater,
                        );
                        if let Some(manifestation) = manifestation {
                            let skill_entity_id = entities.create();
                            updater.insert(skill_entity_id, SkillManifestationComponent::new(
                                skill_entity_id,
                                manifestation),
                            );
                        }

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
                            if distance <= char_comp.calculated_attribs.attack_range.multiply(2.0) {
                                let attack_anim_duration = ElapsedTime(1.0 / char_comp.calculated_attribs.attack_speed.as_f32());
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
                            char_comp.target = None;
                        }
                    } else if let EntityTarget::Pos(target_pos) = target {
                        let distance = nalgebra::distance(&nalgebra::Point::from(char_pos), &target_pos);
                        if distance <= 0.2 {
                            // stop
                            char_comp.set_state(CharState::Idle, char_comp.dir());
                            char_comp.target = None;
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
        for (char_comp, physics_comp) in (&char_state_storage,
                                          &physics_storage).join() {
            if let CharState::Walking(target_pos) = char_comp.state() {
                let dir = (target_pos - char_comp.pos()).normalize();
                let speed = dir * char_comp.calculated_attribs.walking_speed.multiply(600.0 * 0.01);
                let force = speed;
                let body = physics_world.rigid_body_mut(physics_comp.body_handle).unwrap();
                body.set_linear_velocity(body.velocity().linear + force);
            }
        }

        // update character's sprite based on its state
        for (char_id, char_comp, sprite) in (&entities, &mut char_state_storage, &mut sprite_storage).join() {
            let sprite: &mut SpriteRenderDescriptorComponent = sprite;
            // e.g. don't switch to IDLE immediately when prev state is ReceivingDamage let
            //   ReceivingDamage animation play till to the end
            let state: CharState = char_comp.state().clone();
            let prev_state: CharState = char_comp.prev_state().clone();
            let prev_animation_has_ended = sprite.animation_ends_at.has_passed(system_vars.time);
            let prev_animation_must_stop_at_end = match char_comp.prev_state() {
                CharState::Walking(_) => true,
                _ => false,
            };
            let state_has_changed = char_comp.state_has_changed();
            if state_has_changed {
                log::debug!("{:?} state has changed {:?} ==> {:?}",
                    char_id,
                    prev_state,
                    state
                )
            }
            if (state_has_changed && state != CharState::Idle) ||
                (state == CharState::Idle && prev_animation_has_ended) ||
                (state == CharState::Idle && prev_animation_must_stop_at_end) {
                sprite.animation_started = system_vars.time;
                let forced_duration = match &state {
                    CharState::Attacking { attack_ends, .. } => Some(attack_ends.minus(system_vars.time)),
                    // HACK: '100.0', so the first frame is rendered during casting :)
                    CharState::CastingSkill(casting_info) => Some(casting_info.cast_ends.add_seconds(100.0)),
                    _ => None
                };
                sprite.forced_duration = forced_duration;
                sprite.fps_multiplier = if state.is_walking() {
                    char_comp.calculated_attribs.walking_speed.as_f32()
                } else {
                    1.0
                };
                let (sprite_res, action_index) = char_comp.outlook.get_sprite_and_action_index(
                    &system_vars.sprites,
                    &state
                );
                sprite.action_index = action_index;
                sprite.animation_ends_at = system_vars.time.add(
                    forced_duration.unwrap_or_else(|| {
                        let duration = sprite_res
                            .action
                            .actions[action_index]
                            .duration;
                        ElapsedTime(duration)
                    })
                );
            } else if char_comp.went_from_casting_to_idle() {
                // During casting, only the first frame is rendered
                // when casting is finished, we let the animation runs till the end
                sprite.animation_started = system_vars.time.add_seconds(-0.1);
                sprite.forced_duration = None;
                let (sprite_res, action_index) = char_comp.outlook.get_sprite_and_action_index(
                    &system_vars.sprites,
                    &prev_state
                );
                let duration = sprite_res
                    .action
                    .actions[action_index]
                    .duration;
                sprite.animation_ends_at = sprite.animation_started.add_seconds(duration);
            }
            sprite.direction = char_comp.dir();
            char_comp.save_prev_state();
        }
    }
}