use crate::components::skills::skills::FinishSimpleSkillCastComponent;
use crate::configs::DevConfig;
use crate::systems::SystemVariables;
use specs::prelude::*;
use specs::LazyUpdate;

pub struct FinishSimpleSkillCastSystem;

impl<'a> specs::System<'a> for FinishSimpleSkillCastSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, FinishSimpleSkillCastComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::ReadExpect<'a, DevConfig>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (entities, finish_cast_storage, mut sys_vars, dev_config, mut updater): Self::SystemData,
    ) {
        for (finish_cast_comp_id, finish_cast_comp) in (&entities, &finish_cast_storage).join() {
            let cast_data = &finish_cast_comp.finish_cast_data;
            (finish_cast_comp.logic)(cast_data, &entities, &updater, &dev_config, &mut sys_vars);

            updater.remove::<FinishSimpleSkillCastComponent>(finish_cast_comp_id);
        }
    }
}
