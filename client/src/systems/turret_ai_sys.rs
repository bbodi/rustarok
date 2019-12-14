use specs::prelude::*;

use crate::components::char::{
    CharacterStateComponent, TurretComponent, TurretControllerComponent,
};
use crate::components::controller::{ControllerEntityId, LocalPlayerControllerComponent};
use crate::configs::DevConfig;
use crate::systems::minion_ai_sys::MinionAiSystem;
use crate::systems::SystemFrameDurations;
use rustarok_common::common::v2_to_p2;
use rustarok_common::components::char::EntityTarget;
use rustarok_common::components::controller::PlayerIntention;

pub struct TurretAiSystem;

impl TurretAiSystem {}

impl<'a> System<'a> for TurretAiSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, LocalPlayerControllerComponent>,
        ReadStorage<'a, CharacterStateComponent>,
        ReadStorage<'a, TurretControllerComponent>,
        ReadStorage<'a, TurretComponent>,
        WriteExpect<'a, SystemFrameDurations>,
        ReadExpect<'a, DevConfig>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut controller_storage,
            char_state_storage,
            turret_controller_storage,
            turret_storage,
            mut system_benchmark,
            dev_configs,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("TurretAiSystem");
        for (controller_id, controller, _turret) in (
            &entities,
            &mut controller_storage,
            &turret_controller_storage,
        )
            .join()
        {
            let controller_id = ControllerEntityId(controller_id);
            let radius = dev_configs.skills.gaz_turret.turret.attack_range.as_f32() * 100.0;
            let char_state = char_state_storage.get(controller.controlled_entity.into());

            if let Some(char_state) = char_state {
                // at this point, preferred target is an enemy for sure
                let preferred_target_id = turret_storage
                    .get(controller.controlled_entity.into())
                    .unwrap()
                    .preferred_target;
                if let Some(preferred_target_id) = preferred_target_id {
                    if char_state
                        .target
                        .as_ref()
                        .map(|it| match it {
                            EntityTarget::OtherEntity(target_id) => {
                                *target_id != preferred_target_id
                            }
                            _ => true,
                        })
                        .unwrap_or(true)
                    {
                        if let Some(preferred_target) =
                            char_state_storage.get(preferred_target_id.into())
                        {
                            let current_distance = nalgebra::distance(
                                &v2_to_p2(&preferred_target.pos()),
                                &v2_to_p2(&char_state.pos()),
                            );
                            if !preferred_target.state().is_dead() && current_distance < radius {
                                controller.next_action =
                                    Some(PlayerIntention::Attack(preferred_target_id));
                                return;
                            }
                        }
                    }
                }
                // Hack
                let mut current_target_id = None;
                // hack end
                // first check if preferred target is in range

                let current_target_entity = match char_state.target {
                    Some(EntityTarget::OtherEntity(target_id)) => {
                        current_target_id = Some(target_id);
                        char_state_storage.get(target_id.into())
                    }
                    _ => None,
                };
                let no_target_or_dead_or_out_of_range = match current_target_entity {
                    Some(target) => {
                        let current_distance = nalgebra::distance(
                            &v2_to_p2(&target.pos()),
                            &v2_to_p2(&char_state.pos()),
                        );
                        target.state().is_dead() || current_distance > radius
                    }
                    None => true,
                };

                controller.next_action = if no_target_or_dead_or_out_of_range {
                    let maybe_enemy = MinionAiSystem::get_closest_enemy_in_area(
                        &entities,
                        &char_state_storage,
                        &char_state.pos(),
                        radius,
                        char_state.team,
                        controller.controlled_entity,
                    );
                    match maybe_enemy {
                        Some(target_id) => Some(PlayerIntention::Attack(target_id)),
                        None => None,
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
