use specs::prelude::*;

use crate::components::char::CharacterStateComponent;
use crate::components::skills::skills::SkillManifestationComponent;
use crate::systems::{CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::PhysicEngine;

pub struct SkillSystem;

impl<'a> specs::System<'a> for SkillSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, CollisionsFromPrevFrame>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteExpect<'a, PhysicEngine>,
        specs::WriteStorage<'a, SkillManifestationComponent>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut char_storage,
            mut system_vars,
            collisions_resource,
            mut system_benchmark,
            mut physics_world,
            mut skill_storage,
            mut updater,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("SkillSystem");
        for (entity_id, skill) in (&entities, &mut skill_storage).join() {
            skill.update(
                entity_id,
                &collisions_resource.collisions,
                &mut system_vars,
                &entities,
                &mut char_storage,
                &mut physics_world,
                &mut updater,
            );
        }
    }
}
