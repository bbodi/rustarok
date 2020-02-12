use specs::prelude::*;

use crate::components::char::CharacterStateComponent;
use crate::components::skills::skills::{
    SkillManifestationComponent, SkillManifestationUpdateParam,
};
use crate::systems::{CollisionsFromPrevFrame, SystemFrameDurations, SystemVariables};
use crate::PhysicEngine;
use rustarok_common::attack::{ApplyForceComponent, AreaAttackComponent, HpModificationRequest};
use rustarok_common::common::EngineTime;
use rustarok_common::components::char::{AuthorizedCharStateComponent, StaticCharDataComponent};

pub struct SkillSystem;

impl<'a> System<'a> for SkillSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, CharacterStateComponent>,
        ReadStorage<'a, StaticCharDataComponent>,
        WriteStorage<'a, AuthorizedCharStateComponent>,
        WriteExpect<'a, SystemVariables>,
        ReadExpect<'a, EngineTime>,
        WriteExpect<'a, CollisionsFromPrevFrame>,
        WriteExpect<'a, SystemFrameDurations>,
        WriteExpect<'a, PhysicEngine>,
        WriteStorage<'a, SkillManifestationComponent>,
        Write<'a, LazyUpdate>,
        WriteExpect<'a, Vec<HpModificationRequest>>,
        WriteExpect<'a, Vec<AreaAttackComponent>>,
        WriteExpect<'a, Vec<ApplyForceComponent>>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut char_storage,
            static_data_storage,
            mut auth_char_storage,
            mut sys_vars,
            time,
            collisions_resource,
            mut system_benchmark,
            mut physics_world,
            mut skill_storage,
            mut updater,
            mut hp_mod_requests,
            mut area_hp_mod_requests,
            mut pushes,
        ): Self::SystemData,
    ) {
        if !time.can_simulation_run() {
            return;
        }
        let _stopwatch = system_benchmark.start_measurement("SkillSystem");
        for (entity_id, skill) in (&entities, &mut skill_storage).join() {
            skill.update(SkillManifestationUpdateParam::new(
                entity_id,
                &collisions_resource.collisions,
                &mut sys_vars,
                &mut hp_mod_requests,
                &mut area_hp_mod_requests,
                &mut pushes,
                &time,
                &entities,
                &static_data_storage,
                &mut auth_char_storage,
                &mut physics_world,
                &mut updater,
            ));
        }
    }
}
