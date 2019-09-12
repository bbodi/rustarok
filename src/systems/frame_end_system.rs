use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use specs::prelude::*;

pub struct FrameEndSystem;

impl<'a> specs::System<'a> for FrameEndSystem {
    type SystemData = (
        specs::WriteStorage<'a, RenderCommandCollectorComponent>,
        specs::WriteStorage<'a, AudioCommandCollectorComponent>,
    );

    fn run(&mut self, (mut render_commands_storage, mut audio_commands_storage): Self::SystemData) {
        for render_commands in (&mut render_commands_storage).join() {
            render_commands.clear();
        }
        for audio_commands in (&mut audio_commands_storage).join() {
            audio_commands.clear();
        }
    }
}
