use crate::asset::AssetLoader;
use crate::systems::SystemFrameDurations;
use specs::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub struct AudioCommandCollectorComponent {
    sound_commands: Vec<SoundAudioCommand>,
}

struct SoundAudioCommand {
    sound_id: SoundId,
}

impl AudioCommandCollectorComponent {
    pub fn new() -> AudioCommandCollectorComponent {
        AudioCommandCollectorComponent {
            sound_commands: Vec::with_capacity(128),
        }
    }

    pub fn clear(&mut self) {
        self.sound_commands.clear();
    }

    pub fn add_sound_command(&mut self, sound_id: SoundId) {
        self.sound_commands.push(SoundAudioCommand { sound_id });
    }
}

struct Sounds {
    sounds: HashMap<SoundId, sdl2::mixer::Chunk>,
}

impl Sounds {
    pub fn new() -> Sounds {
        Sounds {
            sounds: HashMap::new(),
        }
    }
    pub fn store_wav(&mut self, chunk: sdl2::mixer::Chunk) -> SoundId {
        let id = SoundId(self.sounds.len());
        self.sounds.insert(id, chunk);
        return id;
    }
    pub fn get(&self, sound_id: SoundId) -> &sdl2::mixer::Chunk {
        &self.sounds[&sound_id]
    }
}

pub struct SoundSystem {
    sounds: Sounds,
}

#[derive(Eq, Hash, PartialEq, Copy, Clone)]
pub struct SoundId(usize);

impl SoundSystem {
    pub fn new() -> SoundSystem {
        SoundSystem {
            sounds: Sounds::new(),
        }
    }

    pub fn load_wav(&mut self, path: &str, asset_loader: &AssetLoader) -> Result<SoundId, String> {
        let chunk = asset_loader.load_wav(path)?;
        return Ok(self.sounds.store_wav(chunk));
    }
}

impl<'a> specs::System<'a> for SoundSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, AudioCommandCollectorComponent>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (entities, audio_commands, mut system_benchmark): Self::SystemData) {
        let _stopwatch = system_benchmark.start_measurement("SoundSystem");

        for audio_commands in audio_commands.join() {
            let audio_commands: &AudioCommandCollectorComponent = audio_commands;

            for sound_command in &audio_commands.sound_commands {
                let chunk = self.sounds.get(sound_command.sound_id);
                let sh = sdl2::mixer::Channel::all().play(chunk, 0);
            }
        }
    }
}
