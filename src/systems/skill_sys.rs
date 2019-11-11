use specs::prelude::*;

use crate::components::char::CharacterStateComponent;
use crate::components::skills::skills::{
    SkillManifestationComponent, SkillManifestationUpdateParam,
};
use crate::systems::{CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::PhysicEngine;

pub struct SkillSystem;

impl<'a> System<'a> for SkillSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, CharacterStateComponent>,
        WriteExpect<'a, SystemVariables>,
        WriteExpect<'a, CollisionsFromPrevFrame>,
        WriteExpect<'a, SystemFrameDurations>,
        WriteExpect<'a, PhysicEngine>,
        WriteStorage<'a, SkillManifestationComponent>,
        Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut char_storage,
            mut sys_vars,
            collisions_resource,
            mut system_benchmark,
            mut physics_world,
            mut skill_storage,
            mut updater,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("SkillSystem");
        for (entity_id, skill) in (&entities, &mut skill_storage).join() {
            //            skill.update(SkillManifestationUpdateParam {
            //                self_entity_id: entity_id,
            //                all_collisions_in_world: &collisions_resource.collisions,
            //                sys_vars: &mut sys_vars,
            //                entities: &entities,
            //                char_storage: &mut char_storage,
            //                physics_world: &mut physics_world,
            //                updater: &mut updater,
            //            });
            skill.update(SkillManifestationUpdateParam::new(
                entity_id,
                &collisions_resource.collisions,
                &mut sys_vars,
                &entities,
                &mut char_storage,
                &mut physics_world,
                &mut updater,
            ));
        }
    }
}
