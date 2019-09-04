use crate::components::char::{
    CastingSkillData, CharState, CharacterStateComponent, EntityTarget,
    SpriteRenderDescriptorComponent,
};
use crate::components::controller::{
    CharEntityId, ControllerComponent, ControllerEntityId, EntitiesBelowCursor, PlayerIntention,
    WorldCoords,
};
use crate::components::skills::skill::{SkillTargetType, Skills};
use crate::systems::render_sys::DIRECTION_TABLE;
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::ElapsedTime;
use nalgebra::Vector2;
use specs::prelude::*;

pub struct NextActionApplierSystem;

impl<'a> specs::System<'a> for NextActionApplierSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, SpriteRenderDescriptorComponent>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut char_state_storage,
            mut sprite_storage,
            mut controller_storage,
            system_vars,
            mut system_benchmark,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("NextActionApplierSystem");
        let now = system_vars.time;
        for (controller) in (&mut controller_storage).join() {
            let char_state = char_state_storage.get_mut(controller.controlled_entity.0);

            // the controlled character might have been removed due to death etc
            if let Some(char_state) = char_state {
                controller.next_action_allowed = match controller.next_action {
                    Some(PlayerIntention::MoveTo(pos)) => {
                        char_state.target = Some(EntityTarget::Pos(pos));
                        true
                    }
                    Some(PlayerIntention::Attack(target_entity_id)) => {
                        char_state.target = Some(EntityTarget::OtherEntity(target_entity_id));
                        true
                    }
                    Some(PlayerIntention::MoveTowardsMouse(pos)) => {
                        char_state.target = Some(EntityTarget::Pos(pos));
                        true
                    }
                    Some(PlayerIntention::AttackTowards(_)) => true,
                    Some(PlayerIntention::Casting(skill, is_self_cast, world_coords)) => {
                        NextActionApplierSystem::try_cast_skill(
                            skill,
                            now,
                            char_state,
                            &world_coords,
                            &controller.entities_below_cursor,
                            controller.controlled_entity,
                            is_self_cast,
                        )
                    }
                    None => true,
                }
            }
        }

        // update character's sprite based on its state
        for (char_id, char_comp, sprite) in
            (&entities, &mut char_state_storage, &mut sprite_storage).join()
        {
            let sprite: &mut SpriteRenderDescriptorComponent = sprite;
            // e.g. don't switch to IDLE immediately when prev state is ReceivingDamage let
            //   ReceivingDamage animation play till to the end
            let state: CharState = char_comp.state().clone();
            let prev_state: CharState = char_comp.prev_state().clone();
            let prev_animation_has_ended = sprite.animation_ends_at.is_earlier_than(now);
            let prev_animation_must_stop_at_end = match char_comp.prev_state() {
                CharState::Walking(_) => true,
                _ => false,
            };
            let state_has_changed = char_comp.state_has_changed();
            if state_has_changed {
                log::debug!(
                    "{:?} state has changed {:?} ==> {:?}",
                    char_id,
                    prev_state,
                    state
                )
            }
            if (state_has_changed && state != CharState::Idle)
                || (state == CharState::Idle && prev_animation_has_ended)
                || (state == CharState::Idle && prev_animation_must_stop_at_end)
            {
                sprite.animation_started = now;
                let forced_duration = match &state {
                    CharState::Attacking { .. } => Some(char_comp.attack_delay_ends_at.minus(now)),
                    // HACK: '100.0', so the first frame is rendered during casting :)
                    CharState::CastingSkill(casting_info) => {
                        Some(casting_info.cast_ends.add_seconds(100.0))
                    }
                    _ => None,
                };
                sprite.forced_duration = forced_duration;
                sprite.fps_multiplier = if state.is_walking() {
                    char_comp.calculated_attribs().walking_speed.as_f32()
                } else {
                    1.0
                };
                let (sprite_res, action_index) = char_comp
                    .outlook
                    .get_sprite_and_action_index(&system_vars.assets.sprites, &state);
                sprite.action_index = action_index;
                sprite.animation_ends_at = now.add(forced_duration.unwrap_or_else(|| {
                    let duration = sprite_res.action.actions[action_index].duration;
                    ElapsedTime(duration)
                }));
            } else if char_comp.went_from_casting_to_idle() {
                // During casting, only the first frame is rendered
                // when casting is finished, we let the animation runs till the end
                sprite.animation_started = now.add_seconds(-0.1);
                sprite.forced_duration = None;
                let (sprite_res, action_index) = char_comp
                    .outlook
                    .get_sprite_and_action_index(&system_vars.assets.sprites, &prev_state);
                let duration = sprite_res.action.actions[action_index].duration;
                sprite.animation_ends_at = sprite.animation_started.add_seconds(duration);
            }
            sprite.direction = char_comp.dir();
            char_comp.save_prev_state();
        }
    }
}

impl NextActionApplierSystem {
    pub fn try_cast_skill(
        skill: Skills,
        now: ElapsedTime,
        char_state: &mut CharacterStateComponent,
        world_coords: &WorldCoords,
        entities_below_cursor: &EntitiesBelowCursor,
        self_char_id: CharEntityId,
        is_self_cast: bool,
    ) -> bool {
        if char_state
            .skill_cast_allowed_at
            .entry(skill)
            .or_insert(ElapsedTime(0.0))
            .is_later_than(now)
        {
            return false;
        }
        let (target_pos, target_entity) = if is_self_cast {
            (char_state.pos(), Some(self_char_id))
        } else {
            let target_entity = match skill.get_skill_target_type() {
                SkillTargetType::AnyEntity => entities_below_cursor.get_enemy_or_friend(),
                SkillTargetType::NoTarget => panic!(), /* NoTarget should have been casted already */
                SkillTargetType::Area => None,
                SkillTargetType::OnlyAllyButNoSelf => {
                    entities_below_cursor.get_friend_except(self_char_id)
                }
                SkillTargetType::OnlyAllyAndSelf => entities_below_cursor.get_friend(),
                SkillTargetType::OnlyEnemy => entities_below_cursor.get_enemy(),
                SkillTargetType::OnlySelf => panic!(), /* NoTarget should have been casted already */
            };
            (*world_coords, target_entity)
        };
        let distance = (char_state.pos() - target_pos).magnitude();
        let allowed =
            skill.is_casting_allowed_based_on_target(self_char_id, target_entity, distance);
        let can_move = char_state.can_move(now);
        if allowed && can_move {
            log::debug!("Casting request for '{:?}' was allowed", skill);
            let casting_time_seconds = skill.get_casting_time(&char_state);
            let dir_vector = target_pos - char_state.pos();
            let dir_vector = if dir_vector.x == 0.0 && dir_vector.y == 0.0 {
                v2!(1, 0)
            } else {
                dir_vector.normalize()
            };
            let new_state = CharState::CastingSkill(CastingSkillData {
                target_entity,
                cast_started: now,
                cast_ends: now.add(casting_time_seconds),
                can_move: false,
                skill,
                target_area_pos: match skill.get_skill_target_type() {
                    SkillTargetType::Area => Some(target_pos),
                    _ => None,
                },
                char_to_skill_dir_when_casted: dir_vector,
            });
            let dir = if is_self_cast && target_entity.map(|it| it == self_char_id).is_some() {
                // skill on self, don't change direction
                char_state.dir()
            } else {
                let char_pos = char_state.pos();
                NextActionApplierSystem::determine_dir(&target_pos, &char_pos)
            };
            char_state.set_state(new_state, dir);
            *char_state.skill_cast_allowed_at.get_mut(&skill).unwrap() =
                now.add(skill.get_cast_delay(&char_state));
            return true;
        } else {
            log::debug!(
                "Casting request for '{:?}' was rejected, allowed: {}, can_move: {}",
                skill,
                allowed,
                can_move
            );
            return false;
        }
    }

    pub fn determine_dir(&target_pos: &WorldCoords, pos: &WorldCoords) -> usize {
        let dir_vec = target_pos - pos;
        // "- 90.0"
        // The calculated yaw for the camera are 90 at [0;1] and 180 at [1;0] etc,
        // this calculation gives a different result which is shifted 90 degrees clockwise,
        // so it is 90 at [1;0].
        let dd = dir_vec.x.atan2(dir_vec.y).to_degrees() - 90.0;
        let dd = if dd < 0.0 {
            dd + 360.0
        } else if dd > 360.0 {
            dd - 360.0
        } else {
            dd
        };
        let dir_index = (dd / 45.0 + 0.5) as usize % 8;
        return DIRECTION_TABLE[dir_index];
    }
}
