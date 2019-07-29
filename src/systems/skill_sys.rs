use specs::prelude::*;

use crate::components::char::{CharacterStateComponent, PhysicsComponent};
use crate::components::controller::ControllerComponent;
use crate::components::skills::skill::SkillManifestationComponent;
use crate::components::BrowserClient;
use crate::systems::{CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::PhysicsWorld;

pub struct SkillSystem;

impl<'a> specs::System<'a> for SkillSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, CollisionsFromPrevFrame>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteStorage<'a, SkillManifestationComponent>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            char_storage,
            input_storage,
            browser_client_storage,
            physics_storage,
            mut system_vars,
            collisions_resource,
            mut system_benchmark,
            mut physics_world,
            mut skill_storage,
            mut updater,
        ): Self::SystemData,
    ) {
        let stopwatch = system_benchmark.start_measurement("SkillSystem");
        for (entity_id, skill) in (&entities, &mut skill_storage).join() {
            skill.update(
                entity_id,
                &collisions_resource.collisions,
                &mut system_vars,
                &entities,
                &char_storage,
                &mut physics_world,
                &mut updater,
            );
        }
    }
}
