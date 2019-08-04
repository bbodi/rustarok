use crate::components::char::{CastingSkillData, CharState, CharacterStateComponent, EntityTarget};
use crate::components::controller::{ControllerComponent, PlayerIntention, WorldCoords};
use crate::components::skills::skill::{SkillTargetType, Skills};
use crate::systems::render_sys::DIRECTION_TABLE;
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::ElapsedTime;
use nalgebra::Vector2;
use specs::prelude::*;

pub struct CharacterControlSystem;

impl<'a> specs::System<'a> for CharacterControlSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    // TODO: it is not obvious what is the difference between this, input sys and char_state_sys
    fn run(
        &mut self,
        (
            entities,
        mut char_state_storage,
        controller_storage,
        system_vars,
        mut system_benchmark,
    ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("CharacterControlSystem");
        for (entity_id, controller, char_state) in
            (&entities, &controller_storage, &mut char_state_storage).join()
        {
            // for autocompletion...
            let controller: &ControllerComponent = controller;
            let char_state: &mut CharacterStateComponent = char_state;

            match controller.next_action {
                Some(PlayerIntention::MoveTo(pos)) => {
                    char_state.target = Some(EntityTarget::Pos(pos));
                }
                Some(PlayerIntention::Attack(entity)) => {
                    char_state.target = Some(EntityTarget::OtherEntity(entity))
                }
                Some(PlayerIntention::MoveTowardsMouse(pos)) => {
                    char_state.target = Some(EntityTarget::Pos(pos));
                }
                Some(PlayerIntention::AttackTowards(_)) => {}
                Some(PlayerIntention::Casting(
                    skill,
                    is_self_cast,
                    world_coords,
                    target_entity,
                )) => {
                    CharacterControlSystem::try_cast_skill(
                        skill,
                        system_vars.time,
                        char_state,
                        &world_coords,
                        target_entity,
                        entity_id,
                        is_self_cast,
                    );
                }
                None => {}
            }
        }
    }
}

impl CharacterControlSystem {
    pub fn try_cast_skill(
        skill: Skills,
        now: ElapsedTime,
        char_state: &mut CharacterStateComponent,
        world_coords: &WorldCoords,
        target_entity: Option<Entity>,
        self_id: Entity,
        is_self_cast: bool,
    ) {
        let (target_pos, target_entity) = if is_self_cast {
            (char_state.pos(), Some(self_id))
        } else {
            (*world_coords, target_entity)
        };
        let distance = (char_state.pos() - target_pos).magnitude();
        let allowed = skill.is_casting_allowed(self_id, target_entity, distance);
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
            let dir = if is_self_cast && target_entity.map(|it| it == self_id).is_some() {
                // skill on self, don't change direction
                char_state.dir()
            } else {
                let char_pos = char_state.pos();
                CharacterControlSystem::determine_dir(&target_pos, &char_pos)
            };
            char_state.set_state(new_state, dir);
        } else {
            log::debug!(
                "Casting request for '{:?}' was rejected, allowed: {}, can_move: {}",
                skill,
                allowed,
                can_move
            );
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
