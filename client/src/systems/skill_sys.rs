use specs::prelude::*;

use crate::components::char::CharacterStateComponent;
use crate::components::skills::skills::{
    SkillManifestationComponent, SkillManifestationUpdateParam,
};
use crate::systems::{CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::PhysicEngine;
use rustarok_common::common::EngineTime;
use rustarok_common::components::char::AuthorizedCharStateComponent;

pub struct SkillSystem;

impl<'a> System<'a> for SkillSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, CharacterStateComponent>,
        WriteStorage<'a, AuthorizedCharStateComponent>,
        WriteExpect<'a, SystemVariables>,
        ReadExpect<'a, EngineTime>,
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
            mut auth_char_storage,
            mut sys_vars,
            time,
            collisions_resource,
            mut system_benchmark,
            mut physics_world,
            mut skill_storage,
            mut updater,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("SkillSystem");
        for (entity_id, skill) in (&entities, &mut skill_storage).join() {
            skill.update(SkillManifestationUpdateParam::new(
                entity_id,
                &collisions_resource.collisions,
                &mut sys_vars,
                &time,
                &entities,
                &mut char_storage,
                &mut auth_char_storage,
                &mut physics_world,
                &mut updater,
            ));
        }
    }
}
