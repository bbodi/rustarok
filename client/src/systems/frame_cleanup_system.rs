use specs::prelude::*;

use rustarok_common::common::EngineTime;

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::render::render_command::RenderCommandCollector;
use crate::systems::snapshot_sys::GameSnapshots;
use crate::systems::SystemVariables;

pub struct FrameCleanupSystem;

impl<'a> System<'a> for FrameCleanupSystem {
    type SystemData = (
        WriteStorage<'a, RenderCommandCollector>,
        WriteStorage<'a, AudioCommandCollectorComponent>,
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
