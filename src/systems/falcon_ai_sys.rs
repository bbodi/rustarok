use crate::components::char::{CharacterStateComponent, SpriteRenderDescriptorComponent};
use crate::components::controller::{CharEntityId, WorldCoords};
use crate::configs::DevConfig;
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::{SystemFrameDurations, SystemVariables};
use nalgebra::Vector2;
use specs::prelude::*;

pub struct FalconAiSystem;

pub enum FalconState {
    Follow,
    CarryOwner,
    CarryAlly,
}

#[derive(Component)]
pub struct FalconComponent {
    pub owner_entity_id: CharEntityId,
    pub state: FalconState,
    pub pos: WorldCoords,
    pub acceleration: f32,
}
impl FalconAiSystem {}

impl<'a> specs::System<'a> for FalconAiSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, FalconComponent>,
        specs::WriteStorage<'a, SpriteRenderDescriptorComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::ReadExpect<'a, DevConfig>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut falcon_storage,
            mut sprite_storage,
            char_storage,
            mut system_benchmark,
            system_vars,
            dev_configs,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("FalconAiSystem");
        for (falcon_id, falcon, sprite) in
            (&entities, &mut falcon_storage, &mut sprite_storage).join()
        {
            if let Some(owner) = char_storage.get(falcon.owner_entity_id.0) {
                match falcon.state {
                    FalconState::Follow => {
                        let diff_v = owner.pos() - falcon.pos;
                        if (owner.pos() - falcon.pos).magnitude() > 2.0 {
                            let dir = diff_v.normalize();
                            falcon.acceleration = (falcon.acceleration + system_vars.dt.0 * 0.05)
                                .min(0.03 * owner.calculated_attribs().walking_speed.as_f32());
                            falcon.pos += dir * falcon.acceleration;
                            sprite.direction =
                                NextActionApplierSystem::determine_dir(&owner.pos(), &falcon.pos);
                        } else {
                            if falcon.acceleration < 0.00001 {
                                falcon.acceleration = 0.0;
                            } else {
                                falcon.acceleration -= system_vars.dt.0 * 0.1;
                                let dir = diff_v.normalize();
                                falcon.pos += dir * falcon.acceleration;
                            }
                        }
                    }
                    FalconState::CarryOwner => {}
                    FalconState::CarryAlly => {}
                }
            } else {
                entities.delete(falcon_id);
            }
        }
    }
}
