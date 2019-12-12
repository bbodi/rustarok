use crate::get_current_ms;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::SystemVariables;
use specs::prelude::*;

pub struct ClientFrameEndSystem;

impl<'a> System<'a> for ClientFrameEndSystem {
    type SystemData = (
        WriteStorage<'a, RenderCommandCollector>,
        WriteStorage<'a, AudioCommandCollectorComponent>,
        WriteExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (mut render_commands_storage, mut audio_commands_storage, mut _sys_vars): Self::SystemData,
    ) {
        for render_commands in (&mut render_commands_storage).join() {
            render_commands.clear();
        }
        for audio_commands in (&mut audio_commands_storage).join() {
            audio_commands.clear();
        }
        // when client and server will be separated, client has to update its timer as well
        //        let now = std::time::SystemTime::now();
        //        let now_ms = get_current_ms(now);
        //        sys_vars.update_timers(now_ms);
    }
}

pub struct ServerFrameEndSystem;

impl<'a> System<'a> for ServerFrameEndSystem {
    type SystemData = (WriteExpect<'a, SystemVariables>);

    fn run(&mut self, mut sys_vars: Self::SystemData) {
        let now = std::time::SystemTime::now();
        let now_ms = get_current_ms(now);
        sys_vars.update_timers(now_ms);
    }
}
