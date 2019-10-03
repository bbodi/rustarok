use crate::asset::database::AssetDatabase;
use crate::components::char::{ActionPlayMode, SpriteRenderDescriptorComponent};
use crate::components::controller::CameraComponent;
use crate::configs::DevConfig;
use crate::systems::falcon_ai_sys::FalconComponent;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::render_single_layer_action;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::SystemVariables;
use specs::prelude::*;

pub struct FalconRenderSys;

impl<'a> specs::System<'a> for FalconRenderSys {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, SpriteRenderDescriptorComponent>,
        specs::ReadStorage<'a, FalconComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::ReadExpect<'a, DevConfig>,
        specs::ReadStorage<'a, CameraComponent>,
        specs::WriteStorage<'a, RenderCommandCollector>,
        specs::WriteStorage<'a, AudioCommandCollectorComponent>,
        specs::ReadExpect<'a, AssetDatabase>,
    );

    fn run(
        &mut self,
        (
            entities,
            sprite_storage,
            falcon_storage,
            sys_vars,
            dev_configs,
            camera_storage,
            mut render_commands_storage,
            mut audio_commands_storage,
            asset_db,
        ): Self::SystemData,
    ) {
        for render_commands in (&mut render_commands_storage).join() {
            for (animated_sprite, falcon) in (&sprite_storage, &falcon_storage).join() {
                let _offset = render_single_layer_action(
                    sys_vars.time,
                    &animated_sprite,
                    &sys_vars.assets.sprites.falcon,
                    &falcon.pos,
                    [0, 0],
                    true,
                    1.0,
                    ActionPlayMode::Repeat,
                    &[255, 255, 255, 255],
                    render_commands,
                );
            }
        }
    }
}