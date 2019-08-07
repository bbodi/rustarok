use specs::prelude::*;

use crate::common::v2_to_p2;
use crate::components::char::{CharState, CharacterStateComponent, EntityTarget, PhysicsComponent};
use crate::components::controller::WorldCoords;
use crate::components::skills::skill::SkillManifestationComponent;
use crate::components::{AttackComponent, AttackType};
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::{CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};
use std::collections::HashMap;

pub struct CharacterStateUpdateSystem;

impl<'a> specs::System<'a> for CharacterStateUpdateSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, PhysicEngine>,
        specs::WriteExpect<'a, CollisionsFromPrevFrame>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut physics_storage,
            mut char_state_storage,
            mut system_vars,
            mut physics_world,
            mut collisions_resource,
            mut system_benchmark,
            mut updater,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("CharacterStateUpdateSystem");
        let now = system_vars.time;

        // TODO: HACK
        // I can't get the position of the target entity inside the loop because
        // char_state storage is borrowed as mutable already
        let mut char_positions = HashMap::<Entity, WorldCoords>::new();
        for (char_entity_id, char_comp) in (&entities, &mut char_state_storage).join() {
            char_positions.insert(char_entity_id, char_comp.pos());
        }

        for (char_entity_id, char_comp) in (&entities, &mut char_state_storage).join() {
            // for autocompletion...
            let char_comp: &mut CharacterStateComponent = char_comp;

            // pakold külön componensbe augy a dolgokat, hogy innen be tudjam álltiani a
            // target et None-ra ha az halott, meg a fenti position hack se kelllejn
            if char_comp.hp <= 0 && *char_comp.state() != CharState::Dead {
                log::debug!("Entity has died {:?}", char_entity_id);
                char_comp.set_state(CharState::Dead, char_comp.dir());
                // TODO: implement Death as status?
                // then swap the order, first remove states then apply new ones (to remove the death state in case of resurrect)
                char_comp.statuses.remove_all();
                // remove rigid bodies from the physic simulation
                if let Some(phys_comp) = physics_storage.get(char_entity_id) {
                    collisions_resource.remove_collider_handle(phys_comp.collider_handle);
                    physics_world.bodies.remove(phys_comp.body_handle);
                    physics_storage.remove(char_entity_id);
                }
                continue;
            }
            if *char_comp.state() == CharState::Dead {
                continue;
            }
            char_comp.update_statuses(char_entity_id, &mut system_vars, &entities, &mut updater);

            let char_pos = char_comp.pos();
            match char_comp.state().clone() {
                CharState::CastingSkill(casting_info) => {
                    if casting_info.cast_ends.is_earlier_than(now) {
                        log::debug!("Skill cast has finished: {:?}", casting_info.skill);
                        let skill_pos = if let Some(target_entity) = casting_info.target_entity {
                            Some(char_positions[&target_entity].clone())
                        } else {
                            casting_info.target_area_pos
                        };
                        let manifestation = casting_info.skill.finish_cast(
                            char_entity_id,
                            &char_pos,
                            skill_pos,
                            &casting_info.char_to_skill_dir_when_casted,
                            casting_info.target_entity,
                            &mut physics_world,
                            &mut system_vars,
                            &entities,
                            &mut updater,
                        );
                        if let Some(manifestation) = manifestation {
                            let skill_entity_id = entities.create();
                            updater.insert(
                                skill_entity_id,
                                SkillManifestationComponent::new(skill_entity_id, manifestation),
                            );
                        }

                        char_comp.set_state(CharState::Idle, char_comp.dir());
                    }
                }
                CharState::Attacking {
                    target,
                    damage_occurs_at,
                } => {
                    if damage_occurs_at.is_earlier_than(now) {
                        char_comp.set_state(CharState::Idle, char_comp.dir());
                        system_vars.attacks.push(AttackComponent {
                            src_entity: char_entity_id,
                            dst_entity: target,
                            typ: AttackType::Basic(
                                char_comp.calculated_attribs().attack_damage as u32,
                            ),
                        });
                    }
                }
                _ => {}
            }

            if char_comp.can_move(now) {
                if let Some(target) = &char_comp.target {
                    if let EntityTarget::OtherEntity(target_entity) = target {
                        let target_pos = char_positions.get(target_entity);
                        if let Some(target_pos) = target_pos {
                            let distance = nalgebra::distance(
                                &nalgebra::Point::from(char_pos),
                                &v2_to_p2(&target_pos),
                            );
                            if distance
                                <= char_comp.calculated_attribs().attack_range.as_f32() * 2.0
                            {
                                if char_comp.attack_delay_ends_at.is_earlier_than(now) {
                                    let attack_anim_duration =
                                        1.0 / char_comp.calculated_attribs().attack_speed.as_f32();
                                    let damage_occurs_at =
                                        now.add_seconds(attack_anim_duration / 2.0);
                                    let new_state = CharState::Attacking {
                                        damage_occurs_at,
                                        target: *target_entity,
                                    };
                                    char_comp.set_state(
                                        new_state,
                                        NextActionApplierSystem::determine_dir(
                                            target_pos, &char_pos,
                                        ),
                                    );
                                    let attack_anim_duration = ElapsedTime(
                                        1.0 / char_comp.calculated_attribs().attack_speed.as_f32(),
                                    );
                                    char_comp.attack_delay_ends_at = now.add(attack_anim_duration);
                                } else {
                                    char_comp.set_state(CharState::Idle, char_comp.dir());
                                }
                            } else {
                                // move closer
                                char_comp.set_state(
                                    CharState::Walking(*target_pos),
                                    NextActionApplierSystem::determine_dir(target_pos, &char_pos),
                                );
                            }
                        } else {
                            char_comp.set_state(CharState::Idle, char_comp.dir());
                            char_comp.target = None;
                        }
                    } else if let EntityTarget::Pos(target_pos) = target {
                        let distance = nalgebra::distance(
                            &nalgebra::Point::from(char_pos),
                            &v2_to_p2(target_pos),
                        );
                        if distance <= 0.2 {
                            // stop
                            char_comp.set_state(CharState::Idle, char_comp.dir());
                            char_comp.target = None;
                        } else {
                            // move closer
                            char_comp.set_state(
                                CharState::Walking(*target_pos),
                                NextActionApplierSystem::determine_dir(target_pos, &char_pos),
                            );
                        }
                    }
                } else {
                    // no target and no receieving damage, casting or attacking
                    char_comp.set_state(CharState::Idle, char_comp.dir());
                }
            }
        }
        // apply moving physics here, so that the prev loop does not have to borrow physics_storage
        for (char_comp, physics_comp) in (&char_state_storage, &physics_storage).join() {
            if let CharState::Walking(target_pos) = char_comp.state() {
                if char_comp.can_move(now) {
                    // it is possible that the character is pushed away but stayed in WALKING state (e.g. because of she blocked the the attack)
                    let dir = (target_pos - char_comp.pos()).normalize();
                    let speed =
                        dir * char_comp.calculated_attribs().walking_speed.as_f32() * (60.0 * 0.1);
                    let force = speed;
                    let body = physics_world
                        .bodies
                        .rigid_body_mut(physics_comp.body_handle)
                        .unwrap();
                    body.set_linear_velocity(force);
                }
            }
        }
    }
}
