use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::get_current_ms;
use crate::render::render_command::RenderCommandCollector;
use crate::systems::SystemVariables;
use rustarok_common::common::EngineTime;
use specs::prelude::*;

pub struct ClientFrameEndSystem;

impl<'a> System<'a> for ClientFrameEndSystem {
    type SystemData = (
        WriteStorage<'a, RenderCommandCollector>,
        WriteStorage<'a, AudioCommandCollectorComponent>,
        WriteExpect<'a, EngineTime>,
    );

    fn run(
        &mut self,
        (mut render_commands_storage, mut audio_commands_storage, mut time): Self::SystemData,
    ) {
        for render_commands in (&mut render_commands_storage).join() {
            render_commands.clear();
        }
        for audio_commands in (&mut audio_commands_storage).join() {
            audio_commands.clear();
        }
        let now = std::time::SystemTime::now();
        let now_ms = get_current_ms(now);
        time.update_timers(now_ms);
    }
}

pub struct ServerFrameEndSystem;

impl<'a> System<'a> for ServerFrameEndSystem {
    type SystemData = (WriteExpect<'a, EngineTime>);

    fn run(&mut self, mut time: Self::SystemData) {
        let now = std::time::SystemTime::now();
        let now_ms = get_current_ms(now);
        time.update_timers(now_ms);
    }
}
