#[cfg(test)]
mod tests {
    use crate::components::char::{CharacterEntityBuilder, Team};
    use crate::components::controller::CharEntityId;
    use crate::configs::{AppConfig, DevConfig};
    use crate::consts::{JobId, JobSpriteId};
    use crate::register_systems;
    use crate::runtime_assets::audio::Sounds;
    use crate::runtime_assets::ecs::create_ecs_world;
    use crate::runtime_assets::graphic::Texts;
    use crate::runtime_assets::map::PhysicEngine;
    use crate::systems::render::render_command::RenderCommandCollector;
    use crate::systems::{
        CollisionsFromPrevFrame, RenderMatrices, Sex, Sprites, SystemFrameDurations,
        SystemVariables,
    };
    use log::LevelFilter;
    use nalgebra::Vector2;
    use specs::prelude::*;
    use std::collections::HashMap;

    pub fn setup_ecs_world<'a, 'b>() -> (specs::World, specs::Dispatcher<'a, 'b>) {
        let config = AppConfig::new().expect("Could not load config file ('config.toml')");
        simple_logging::log_to_stderr(LevelFilter::Trace);

        let mut ecs_world = create_ecs_world();

        // TODO: can I remove render_matrices from system vars?
        let fov = 0.638;
        let render_matrices = RenderMatrices::new(fov);

        let system_vars = SystemVariables::new(
            Sprites::new_for_test(),
            Texts::new_for_test(),
            render_matrices,
            HashMap::new(),
            HashMap::new(),
            vec![],
            Sounds::new_for_test(),
        );

        let ecs_dispatcher = register_systems(None, None, None, true);

        ecs_world.add_resource(system_vars);
        ecs_world.add_resource(DevConfig::new().unwrap());
        ecs_world.add_resource(RenderCommandCollector::new());
        ecs_world.add_resource(CollisionsFromPrevFrame {
            collisions: HashMap::new(),
        });

        ecs_world.add_resource(PhysicEngine::new());
        ecs_world.add_resource(SystemFrameDurations(HashMap::new()));
        return (ecs_world, ecs_dispatcher);
    }

    #[test]
    fn it_works() {
        let (mut ecs_world, mut ecs_dispatcher) = setup_ecs_world();

        let char_entity_id = CharEntityId(ecs_world.create_entity().build());
        {
            let updater = &ecs_world.read_resource::<LazyUpdate>();
            let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
            let dev_configs = &ecs_world.read_resource::<DevConfig>();
            CharacterEntityBuilder::new(char_entity_id, "test char")
                .insert_sprite_render_descr_component(updater)
                .physics(v2!(10, 10), physics_world, |builder| {
                    builder
                        .collision_group(Team::Right.get_collision_group())
                        .circle(1.0)
                })
                .char_state(updater, dev_configs, |ch| {
                    ch.outlook_player(Sex::Male, JobSpriteId::from_job_id(JobId::CRUSADER), 0)
                        .job_id(JobId::CRUSADER)
                        .team(Team::Right)
                });
        }

        for i in 0..1000 {
            ecs_dispatcher.dispatch(&ecs_world.res);
            ecs_world.maintain();
        }
    }
}
