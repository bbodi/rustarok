use crate::components::char::{ActionPlayMode, SpriteRenderDescriptorComponent};
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::render_single_layer_action;
use crate::systems::falcon_ai_sys::FalconComponent;
use crate::systems::SystemVariables;
use rustarok_common::common::EngineTime;
use specs::prelude::*;

pub struct FalconRenderSys;

impl<'a> System<'a> for FalconRenderSys {
    type SystemData = (
        ReadStorage<'a, SpriteRenderDescriptorComponent>,
        ReadStorage<'a, FalconComponent>,
        ReadExpect<'a, SystemVariables>,
        ReadExpect<'a, EngineTime>,
        WriteExpect<'a, RenderCommandCollector>,
    );

    fn run(
        &mut self,
        (sprite_storage, falcon_storage, sys_vars, time, mut render_commands): Self::SystemData,
    ) {
        for (animated_sprite, falcon) in (&sprite_storage, &falcon_storage).join() {
            let _offset = render_single_layer_action(
                time.now(),
                &animated_sprite,
                &sys_vars.assets.sprites.falcon,
                &falcon.pos,
                [0, 0],
                true,
                1.0,
                ActionPlayMode::Repeat,
                &[255, 255, 255, 255],
                &mut render_commands,
            );
        }
    }
}
