use specs::prelude::*;

use crate::common::v2_to_p2;
use crate::components::char::{
    CharState, CharacterStateComponent, EntityTarget, NpcComponent, PhysicsComponent,
};
use crate::components::controller::{CharEntityId, WorldCoords};
use crate::components::skills::skill::SkillManifestationComponent;
use crate::components::status::death_status::DeathStatus;
use crate::components::{AttackComponent, AttackType, DamageDisplayType};
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::{CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};
use std::collections::HashMap;

pub struct CharacterStateUpdateSystem;

impl<'a> specs::System<'a> for CharacterStateUpdateSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, NpcComponent>,
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
            npc_storage,
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
        let mut char_positions = HashMap::<CharEntityId, WorldCoords>::new();
        for (char_entity_id, char_comp) in (&entities, &mut char_state_storage).join() {
            let char_entity_id = CharEntityId(char_entity_id);
            char_positions.insert(char_entity_id, char_comp.pos());
        }

        for (char_entity_id, char_comp) in (&entities, &mut char_state_storage).join() {
            let char_entity_id = CharEntityId(char_entity_id);
            // for autocompletion...
            let char_comp: &mut CharacterStateComponent = char_comp;

            // pakold külön componensbe augy a dolgokat, hogy innen be tudjam álltiani a
            // target et None-ra ha az halott, meg a fenti position hack se kelllejn
            let is_dead = *char_comp.state() == CharState::Dead;
            if char_comp.hp <= 0 && !is_dead {
                log::debug!("Entity has died {:?}", char_entity_id);
                char_comp.set_state(CharState::Dead, char_comp.dir());
                char_comp.statuses.remove_all();
                char_comp.statuses.add(DeathStatus::new(
                    system_vars.time,
                    npc_storage.get(char_entity_id.0).is_some(),
                ));
                // remove rigid bodies from the physic simulation
                if let Some(phys_comp) = physics_storage.get(char_entity_id.0) {
                    collisions_resource.remove_collider_handle(phys_comp.collider_handle);
                    physics_world.bodies.remove(phys_comp.body_handle);
                    physics_storage.remove(char_entity_id.0);
                }
                continue;
            } else if is_dead && npc_storage.get(char_entity_id.0).is_some() {
                let remove_char_at = char_comp
                    .statuses
                    .get_status::<_, DeathStatus, _>(|status| status.remove_char_at)
                    .unwrap();
                if remove_char_at.has_already_passed(system_vars.time) {
                    entities.delete(char_entity_id.0).unwrap();
                }
                continue;
            }

            char_comp.update_statuses(char_entity_id, &mut system_vars, &entities, &mut updater);

            if *char_comp.state() == CharState::Dead {
                continue;
            }

            let char_pos = char_comp.pos();
            match char_comp.state().clone() {
                CharState::CastingSkill(casting_info) => {
                    if casting_info.cast_ends.has_already_passed(now) {
                        log::debug!("Skill cast has finished: {:?}", casting_info.skill);
                        let skill_pos = if let Some(target_entity) = casting_info
                            .target_entity
                            .and_then(|it| char_positions.get(&it))
                        {
                            Some(target_entity.clone())
                        } else {
                            casting_info.target_area_pos
                        };
                        let manifestation = casting_info.skill.get_definition().finish_cast(
                            char_entity_id,
                            char_comp,
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
                    if damage_occurs_at.has_already_passed(now) {
                        char_comp.set_state(CharState::Idle, char_comp.dir());
                        system_vars.attacks.push(AttackComponent {
                            src_entity: char_entity_id,
                            dst_entity: target,
                            typ: AttackType::Basic(
                                char_comp.calculated_attribs().attack_damage as u32,
                                DamageDisplayType::SingleNumber,
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
                                if char_comp.attack_delay_ends_at.has_already_passed(now) {
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
