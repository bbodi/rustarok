use specs::prelude::*;

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::render::render_command::RenderCommandCollector;

pub struct FrameCleanupSystem;

impl<'a> System<'a> for FrameCleanupSystem {
    type SystemData = (
        WriteExpect<'a, RenderCommandCollector>,
        WriteExpect<'a, AudioCommandCollectorComponent>,
    );

    fn run(&mut self, (mut render_commands, mut audio_commands): Self::SystemData) {
        render_commands.clear();
        audio_commands.clear();
    }
}
