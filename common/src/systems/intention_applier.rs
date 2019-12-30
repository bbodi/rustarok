use crate::common::{v2, EngineTime};
use crate::components::char::{AuthorizedCharStateComponent, EntityTarget};
use crate::components::controller::{ControllerComponent, PlayerIntention};
use specs::prelude::*;

pub struct ControllerIntentionToCharTarget;

impl ControllerIntentionToCharTarget {
    pub fn controller_intention_to_char_target(
        controller: &ControllerComponent,
        char_state_storage: &mut specs::WriteStorage<AuthorizedCharStateComponent>,
    ) {
        if let Some(controlled_entity) = controller.controlled_entity {
            let auth_char = char_state_storage.get_mut(controlled_entity.into());

            // the controlled character might have been removed due to death etc
            if let Some(auth_char) = auth_char {
                // TODO2
                //                if char_state.statuses.can_be_controlled() == false {
                //                    continue;
                //                }
                dbg!(&controller.intention);
                match controller.intention {
                    Some(PlayerIntention::MoveTo(pos)) => {
                        auth_char.target = Some(EntityTarget::Pos(pos))
                    }
                    Some(PlayerIntention::Attack(target_entity_id)) => {
                        auth_char.target = Some(EntityTarget::OtherEntity(target_entity_id));
                    }
                    Some(PlayerIntention::MoveTowardsMouse(pos)) => {
                        auth_char.target = Some(EntityTarget::Pos(pos));
                    }
                    Some(PlayerIntention::AttackTowards(pos)) => {
                        auth_char.target = Some(EntityTarget::PosWhileAttacking(pos, None));
                    }
                    None => {} // TODO2
                               //                    Some(PlayerIntention::Casting(skill, is_self_cast, mouse_world_pos)) => {
                               //                        NextActionApplierSystem::try_cast_skill(
                               //                            skill,
                               //                            now,
                               //                            &dev_configs,
                               //                            char_state,
                               //                            &mouse_world_pos,
                               //                            &controller.entities_below_cursor,
                               //                            controller.controlled_entity,
                               //                            is_self_cast,
                               //                        )
                               //                    }
                };
            }
        }
    }

    //    pub fn try_cast_skill(
    //        skill: Skills,
    //        now: ElapsedTime,
    //        configs: &DevConfig,
    //        char_state: &mut CharacterStateComponent,
    //        mouse_world_pos: &Vec2,
    //        entities_below_cursor: &EntitiesBelowCursor,
    //        self_char_id: CharEntityId,
    //        is_self_cast: bool,
    //    ) -> bool {
    //        if char_state
    //            .skill_cast_allowed_at
    //            .entry(skill)
    //            .or_insert(ElapsedTime(0.0))
    //            .has_not_passed_yet(now)
    //        {
    //            return true;
    //        }
    //        let skill_def = skill.get_definition();
    //        let skill_cast_attrs = skill.get_cast_attributes(configs, char_state);
    //        let (target_pos, target_entity) = if is_self_cast {
    //            (char_state.pos(), Some(self_char_id))
    //        } else {
    //            let target_entity = match skill_def.get_skill_target_type() {
    //                SkillTargetType::AnyEntity => entities_below_cursor.get_enemy_or_friend(),
    //                SkillTargetType::NoTarget => None,
    //                SkillTargetType::Area => None,
    //                SkillTargetType::Directional => None,
    //                SkillTargetType::OnlyAllyButNoSelf => {
    //                    entities_below_cursor.get_friend_except(self_char_id)
    //                }
    //                SkillTargetType::OnlyAllyAndSelf => entities_below_cursor.get_friend(),
    //                SkillTargetType::OnlyEnemy => entities_below_cursor.get_enemy(),
    //            };
    //            (*mouse_world_pos, target_entity)
    //        };
    //        let distance = (char_state.pos() - target_pos).magnitude();
    //        let allowed = Skills::is_casting_allowed_based_on_target(
    //            skill_def.get_skill_target_type(),
    //            skill_cast_attrs.casting_range,
    //            self_char_id,
    //            target_entity,
    //            distance,
    //        );
    //        let can_move = char_state.can_cast(now);
    //        if allowed && can_move {
    //            log::debug!("Casting request for '{:?}' was allowed", skill);
    //            let casting_time_seconds = skill_cast_attrs.casting_time;
    //            let (target_pos, dir_vector) = Skills::limit_vector_into_range(
    //                &char_state.pos(),
    //                &target_pos,
    //                skill_cast_attrs.casting_range,
    //            );
    //            let new_state = CharState::CastingSkill(CastingSkillData {
    //                target_entity,
    //                cast_started: now,
    //                cast_ends: now.add(casting_time_seconds),
    //                can_move: false,
    //                skill,
    //                target_area_pos: match skill_def.get_skill_target_type() {
    //                    SkillTargetType::Area | SkillTargetType::Directional => Some(target_pos),
    //                    _ => None,
    //                },
    //                char_to_skill_dir_when_casted: dir_vector,
    //            });
    //            let dir = if is_self_cast || target_entity.map(|it| it == self_char_id).is_some() {
    //                // skill on self, don't change direction
    //                char_state.dir()
    //            } else {
    //                let char_pos = char_state.pos();
    //                NextActionApplierSystem::determine_dir(&target_pos, &char_pos)
    //            };
    //            char_state.set_state(new_state, dir);
    //            *char_state.skill_cast_allowed_at.get_mut(&skill).unwrap() =
    //                now.add(skill_cast_attrs.cast_delay);
    //            return false;
    //        } else {
    //            log::debug!(
    //                "Casting request for '{:?}' was rejected, allowed: {}, can_cast: {}",
    //                skill,
    //                allowed,
    //                can_move
    //            );
    //            return !can_move; // try to repeat casting only when it was interrupted, but not when the target was invalid
    //        }
    //    }
}
