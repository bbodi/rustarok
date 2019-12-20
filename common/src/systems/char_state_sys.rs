use specs::prelude::*;

use crate::common::{v2_to_p2, ElapsedTime, EngineTime, Vec2};
use crate::components::char::{
    AuthorizedCharStateComponent, CharDir, CharEntityId, CharState, EntityTarget, Team,
};
use std::collections::HashMap;

pub struct CharacterStateUpdateSystem;

impl<'a> System<'a> for CharacterStateUpdateSystem {
    type SystemData = (
        Entities<'a>,
        //        ReadStorage<'a, NpcComponent>,
        WriteStorage<'a, AuthorizedCharStateComponent>,
        ReadExpect<'a, EngineTime>,
        Write<'a, LazyUpdate>,
    );

    fn run(&mut self, (entities, mut char_state_storage, time, mut updater): Self::SystemData) {
        let now = time.now();

        // TODO: HACK
        // I can't get the position of the target entity inside the loop because
        // char_state storage is borrowed as mutable already
        // TODO2
        //        let all_char_data = {
        //            let mut char_positions = HashMap::<CharEntityId, (Vec2, Team)>::new();
        //            for (char_entity_id, char_comp) in (&entities, &char_state_storage).join() {
        //                // TODO2
        //                //                if char_comp.state().is_dead() {
        //                //                    continue;
        //                //                }
        //                let char_entity_id = CharEntityId::new(char_entity_id);
        //                char_positions.insert(char_entity_id, (char_comp.pos(), char_comp.team));
        //            }
        //            char_positions
        //        };

        for (char_entity_id, char_comp) in (&entities, &mut char_state_storage).join() {
            let char_entity_id = CharEntityId::new(char_entity_id);
            // pakold külön componensbe augy a dolgokat, hogy innen be tudjam álltiani a
            // target et None-ra ha az halott, meg a fenti position hack se kelllejn
            // TODO2
            //            let is_dead = *char_comp.state() == CharState::Dead;
            //            if char_comp.hp <= 0 && !is_dead {
            //                log::debug!("Entity has died {:?}", char_entity_id);
            //                char_comp.set_state(CharState::Dead, char_comp.dir());
            //                char_comp.statuses.remove_all();
            //                char_comp
            //                    .statuses
            //                    .add(StatusEnum::DeathStatus(DeathStatus::new(
            //                        time.now(),
            //                        npc_storage.get(char_entity_id.into()).is_some(),
            //                    )));
            //                // remove rigid bodies from the physic simulation
            //                collisions_resource.remove_collider_handle(char_comp.collider_handle);
            //                physics_world.bodies.remove(char_comp.body_handle);
            //                continue;
            //            } else if is_dead && npc_storage.get(char_entity_id.into()).is_some() {
            //                if let StatusEnum::DeathStatus(status) = char_comp
            //                    .statuses
            //                    .get_status(StatusEnumDiscriminants::DeathStatus)
            //                    .unwrap()
            //                {
            //                    if status.remove_char_at.has_already_passed(time.now()) {
            //                        entities.delete(char_entity_id.into()).unwrap();
            //                    }
            //                    continue;
            //                }
            //            }

            // TODO2
            //            char_comp.update_statuses(
            //                char_entity_id,
            //                &mut sys_vars,
            //                &time,
            //                &entities,
            //                &mut updater,
            //                &mut physics_world,
            //            );

            // TODO2
            //            if *char_comp.state() == CharState::Dead {
            //                continue;
            //            }

            let char_pos = char_comp.pos();
            // TODO: why clone?
            // TODO2
            //            match char_comp.state().clone() {

            //                CharState::CastingSkill(casting_info) => {
            //                    if casting_info.cast_ends.has_already_passed(now) {
            //                        log::debug!("Skill cast has finished: {:?}", casting_info.skill);
            //                        let skill_pos = if let Some(target_entity) = casting_info
            //                            .target_entity
            //                            .and_then(|it| all_char_data.get(&it))
            //                        {
            //                            Some(target_entity.0.clone())
            //                        } else {
            //                            casting_info.target_area_pos
            //                        };
            //                        sys_vars.just_finished_skill_casts.push(FinishCast {
            //                            skill: casting_info.skill,
            //                            caster_pos: char_pos,
            //                            caster_entity_id: char_entity_id,
            //                            skill_pos,
            //                            char_to_skill_dir: casting_info.char_to_skill_dir_when_casted,
            //                            target_entity: casting_info.target_entity,
            //                            caster_team: char_comp.team,
            //                        });
            //
            //                        char_comp.set_state(CharState::Idle, char_comp.dir());
            //                    }
            //                }
            // TODO2
            //                CharState::Attacking {
            //                    target,
            //                    damage_occurs_at,
            //                    basic_attack,
            //                } => {
            //                    if damage_occurs_at.has_already_passed(now) {
            //                        char_comp.set_state(CharState::Idle, char_comp.dir());
            //                        if let Some(target_pos) = all_char_data.get(&target) {
            //                            if let Some(manifestation) = basic_attack.finish_attack(
            //                                char_comp.calculated_attribs(),
            //                                char_entity_id,
            //                                char_pos,
            //                                target_pos.0,
            //                                target,
            //                                &mut sys_vars,
            //                                &time,
            //                            ) {
            //                                let skill_manifest_id = entities.create();
            //                                updater.insert(
            //                                    skill_manifest_id,
            //                                    SkillManifestationComponent::new(
            //                                        skill_manifest_id,
            //                                        manifestation,
            //                                    ),
            //                                );
            //                            }
            //                        } else {
            //                            // target might have died
            //                        }
            //                    }
            //                }
            //                _ => {}
            //            }

            if true {
                /*char_comp.can_move(now) {*/
                if let Some(target) = &char_comp.target.clone() {
                    // TODO2
                    //                    if let EntityTarget::PosWhileAttacking(pos, current_target) = target {
                    //                        // hack end
                    //                        let current_target_entity = match current_target {
                    //                            Some(target_id) => all_char_data.get(target_id),
                    //                            _ => None,
                    //                        };
                    //                        let no_target_or_dead_or_out_of_range = match current_target_entity {
                    //                            Some((pos, _team)) => {
                    //                                let current_distance = nalgebra::distance(
                    //                                    &v2_to_p2(&pos),
                    //                                    &v2_to_p2(&char_comp.pos()),
                    //                                );
                    //                                current_distance > 10.0
                    //                            }
                    //                            None => true,
                    //                        };
                    //                        if no_target_or_dead_or_out_of_range {
                    //                            let maybe_enemy = CharacterStateUpdateSystem::get_closest_enemy_in_area(
                    //                                &all_char_data,
                    //                                &char_comp.pos(),
                    //                                10.0,
                    //                                char_comp.team,
                    //                                char_entity_id,
                    //                            );
                    //                            char_comp.target =
                    //                                Some(EntityTarget::PosWhileAttacking(*pos, maybe_enemy));
                    //                            CharacterStateUpdateSystem::act_based_on_target(
                    //                                now,
                    //                                &all_char_data,
                    //                                char_comp,
                    //                                &EntityTarget::Pos(*pos),
                    //                            )
                    //                        } else {
                    //                            // there is an active target, move closer or attack it
                    //                            CharacterStateUpdateSystem::act_based_on_target(
                    //                                now,
                    //                                &all_char_data,
                    //                                char_comp,
                    //                                &EntityTarget::OtherEntity(current_target.unwrap()),
                    //                            )
                    //                        }
                    //                    } else {
                    CharacterStateUpdateSystem::act_based_on_target(
                        now, //                        &all_char_data,
                        char_comp, target,
                    )
                //                    }
                } else {
                    // no target and no receieving damage, casting or attacking
                    char_comp.set_state(CharState::Idle, char_comp.dir());
                }
            }
        }

        // TODO: into a system
        // apply moving physics here, so that the prev loop does not have to borrow physics_storage
        for char_comp in (&mut char_state_storage).join() {
            if let CharState::Walking(target_pos) = char_comp.state() {
                if true
                /*TODO2 char_comp.can_move(now)*/
                {
                    // it is possible that the character is pushed away but stayed in WALKING state (e.g. because of she blocked the attack)
                    let dir = (target_pos - char_comp.pos()).normalize();
                    // 100% movement speed = 5 units/second
                    // TODO2
                    //                    let force =
                    //                        dir * char_comp.calculated_attribs().movement_speed.as_f32() * (5.0);
                    char_comp.add_pos(dir * 0.1);
                    log::debug!(
                        "tick: {}, x: {}, y: {}",
                        time.tick,
                        char_comp.pos().x,
                        char_comp.pos().y,
                    );
                }
            }
        }
    }
}

impl CharacterStateUpdateSystem {
    pub fn get_closest_enemy_in_area(
        char_positions: &HashMap<CharEntityId, (Vec2, Team)>,
        center: &Vec2,
        radius: f32,
        self_team: Team,
        except: CharEntityId,
    ) -> Option<CharEntityId> {
        let mut ret = None;
        let mut distance = 2000.0;
        let center = v2_to_p2(center);
        for (char_id, (pos, team)) in char_positions {
            if *char_id == except
                || !team.is_enemy_to(self_team)
                || (pos.x - center.x).abs() > radius
            {
                continue;
            }
            let current_distance = nalgebra::distance(&center, &v2_to_p2(&pos));
            if current_distance <= radius && current_distance < distance {
                distance = current_distance;
                ret = Some(*char_id);
            }
        }
        return ret;
    }

    fn act_based_on_target(
        now: ElapsedTime,
        //        char_positions: &HashMap<CharEntityId, (Vec2, Team)>,
        char_comp: &mut AuthorizedCharStateComponent,
        target: &EntityTarget,
    ) {
        let char_pos = char_comp.pos();
        match target {
            EntityTarget::OtherEntity(target_entity) => {
                // TODO2
                //                let target_pos = char_positions.get(target_entity);
                //                if let Some((target_pos, _team)) = target_pos {
                //                    let distance = nalgebra::distance(
                //                        &nalgebra::Point::from(char_pos),
                //                        &v2_to_p2(&target_pos),
                //                    );
                //
                //                    if distance <= char_comp.calculated_attribs().attack_range.as_f32() * 2.0 {
                //                        if char_comp.attack_delay_ends_at.has_already_passed(now) {
                //                            let attack_anim_duration =
                //                                1.0 / char_comp.calculated_attribs().attack_speed.as_f32();
                //                            let damage_occurs_at = now.add_seconds(attack_anim_duration / 2.0);
                //                            let new_state = CharState::Attacking {
                //                                damage_occurs_at,
                //                                target: *target_entity,
                //                                basic_attack: char_comp.basic_attack_type.clone(),
                //                            };
                //                            char_comp.set_state(
                //                                new_state,
                //                                CharDir::determine_dir(target_pos, &char_pos),
                //                            );
                //                            let attack_anim_duration = ElapsedTime(
                //                                1.0 / char_comp.calculated_attribs().attack_speed.as_f32(),
                //                            );
                //                            char_comp.attack_delay_ends_at = now.add(attack_anim_duration);
                //                        } else {
                //                            char_comp.set_state(CharState::Idle, char_comp.dir());
                //                        }
                //                    } else {
                //                        //                     move closer
                //                        char_comp.set_state(
                //                            CharState::Walking(*target_pos),
                //                            CharDir::determine_dir(target_pos, &char_pos),
                //                        );
                //                    }
                //                } else {
                char_comp.set_state(CharState::Idle, char_comp.dir());
                char_comp.target = None;
                //                }
            }
            EntityTarget::Pos(target_pos) => {
                let distance =
                    nalgebra::distance(&nalgebra::Point::from(char_pos), &v2_to_p2(target_pos));
                if distance <= 0.2 {
                    // stop
                    char_comp.set_state(CharState::Idle, char_comp.dir());
                    char_comp.target = None;
                } else {
                    // move closer
                    char_comp.set_state(
                        CharState::Walking(*target_pos),
                        CharDir::determine_dir(target_pos, &char_pos),
                    );
                }
            }
            EntityTarget::PosWhileAttacking(_pos, _current_target) => {}
        }
    }
}
