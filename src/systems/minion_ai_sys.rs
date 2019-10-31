use crate::common::{v2, v2_to_p2, Vec2};
use crate::components::char::{CharacterStateComponent, EntityTarget, Team};
use crate::components::controller::{
    CharEntityId, ControllerComponent, ControllerEntityId, PlayerIntention,
};
use crate::components::MinionComponent;
use crate::systems::SystemFrameDurations;
use specs::prelude::*;

pub struct MinionAiSystem;

impl MinionAiSystem {
    // from left to right
    pub const CHECKPOINTS: [[i32; 2]; 6] = [
        [245, -204], // right gate
        [175, -204], // right side of fountain
        [156, -220], // top of fountain
        [156, -188], // bottom of fountain
        [136, -204], // left side of fountain
        [64, -204],  // left gate
    ];

    pub fn get_closest_enemy_in_area(
        entities: &Entities,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        center: &Vec2,
        radius: f32,
        self_team: Team,
        except: CharEntityId,
    ) -> Option<CharEntityId> {
        let mut ret = None;
        let mut distance = 2000.0;
        let center = v2_to_p2(center);
        for (entity_id, char_state) in (entities, char_state_storage).join() {
            let entity_id = CharEntityId(entity_id);
            let pos = char_state.pos();
            if entity_id == except
                || !char_state.team.is_enemy_to(self_team)
                || char_state.state().is_dead()
                || (pos.x - center.x).abs() > radius
            {
                continue;
            }
            let current_distance = nalgebra::distance(&center, &v2_to_p2(&pos));
            if current_distance <= radius && current_distance < distance {
                distance = current_distance;
                ret = Some(entity_id);
            }
        }
        return ret;
    }
}

impl<'a> specs::System<'a> for MinionAiSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::ReadStorage<'a, MinionComponent>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut controller_storage,
            char_state_storage,
            minion_storage,
            mut system_benchmark,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("MinionAiSystem");
        for (controller_id, controller, _minion) in
            (&entities, &mut controller_storage, &minion_storage).join()
        {
            let controller_id = ControllerEntityId(controller_id);
            let char_state = char_state_storage.get(controller.controlled_entity.0);

            if let Some(char_state) = char_state {
                // Hack
                let mut current_target_id = None;
                // hack end
                let current_target_entity = match char_state.target {
                    Some(EntityTarget::OtherEntity(target_id)) => {
                        current_target_id = Some(target_id);
                        char_state_storage.get(target_id.0)
                    }
                    _ => None,
                };
                let no_target_or_dead_or_out_of_range = match current_target_entity {
                    Some(target) => {
                        let current_distance = nalgebra::distance(
                            &v2_to_p2(&target.pos()),
                            &v2_to_p2(&char_state.pos()),
                        );
                        target.state().is_dead() || current_distance > 10.0
                    }
                    None => true,
                };

                controller.next_action = if no_target_or_dead_or_out_of_range {
                    let maybe_enemy = MinionAiSystem::get_closest_enemy_in_area(
                        &entities,
                        &char_state_storage,
                        &char_state.pos(),
                        10.0,
                        char_state.team,
                        controller.controlled_entity,
                    );
                    match maybe_enemy {
                        Some(target_id) => Some(PlayerIntention::Attack(target_id)),
                        None => {
                            let next_checkpoint = if char_state.team == Team::Right {
                                let mut next_checkpoint = MinionAiSystem::CHECKPOINTS[5];
                                for checkpoint in MinionAiSystem::CHECKPOINTS.iter() {
                                    if checkpoint[0] < char_state.pos().x as i32 {
                                        next_checkpoint = *checkpoint;
                                        break;
                                    }
                                }
                                next_checkpoint
                            } else {
                                let mut next_checkpoint = MinionAiSystem::CHECKPOINTS[0];
                                for checkpoint in MinionAiSystem::CHECKPOINTS.iter().rev() {
                                    if checkpoint[0] > char_state.pos().x as i32 {
                                        next_checkpoint = *checkpoint;
                                        break;
                                    }
                                }
                                next_checkpoint
                            };
                            Some(PlayerIntention::MoveTo(v2(
                                next_checkpoint[0] as f32,
                                next_checkpoint[1] as f32,
                            )))
                        }
                    }
                } else {
                    Some(PlayerIntention::Attack(current_target_id.unwrap()))
                }
            } else {
                // the char might have died, remove the controller entity
                entities.delete(controller_id.0).expect("");
            }
        }
    }
}
