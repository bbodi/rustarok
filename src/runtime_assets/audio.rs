use crate::asset::AssetLoader;
use crate::systems::sound_sys::{SoundChunkStore, SoundId, SoundSystem, DUMMY_SOUND_ID};

pub struct Sounds {
    pub attack: SoundId,
    pub stun: SoundId,
    pub heal: SoundId,
    pub firewall: SoundId,
}

pub fn init_audio_and_load_sounds(
    sdl_context: &sdl2::Sdl,
    asset_loader: &AssetLoader,
) -> (Option<SoundSystem>, Sounds) {
    return if let Ok(sdl_audio) = sdl_context.audio() {
        init_audio();
        let mut sound_store = SoundChunkStore::new();
        let sounds = load_sounds(&asset_loader, &mut sound_store);
        let sound_system = SoundSystem::new(sdl_audio, sound_store);
        (Some(sound_system), sounds)
    } else {
        (
            None,
            Sounds {
                attack: DUMMY_SOUND_ID,
                stun: DUMMY_SOUND_ID,
                heal: DUMMY_SOUND_ID,
                firewall: DUMMY_SOUND_ID,
            },
        )
    };
}

fn init_audio() {
    let frequency = sdl2::mixer::DEFAULT_FREQUENCY;
    let format = sdl2::mixer::DEFAULT_FORMAT; // signed 16 bit samples, in little-endian byte order
    let channels = sdl2::mixer::DEFAULT_CHANNELS; // Stereo
    let chunk_size = 1_024;
    sdl2::mixer::open_audio(frequency, format, channels, chunk_size);
    let _mixer_context = sdl2::mixer::init(
        sdl2::mixer::InitFlag::MP3
            | sdl2::mixer::InitFlag::FLAC
            | sdl2::mixer::InitFlag::MOD
            | sdl2::mixer::InitFlag::OGG,
    )
    .unwrap();
    sdl2::mixer::allocate_channels(4);
    sdl2::mixer::Channel::all().set_volume(16);
}

fn load_sounds(asset_loader: &AssetLoader, chunk_store: &mut SoundChunkStore) -> Sounds {
    // TODO: use dummy sound instead of unwrap
    let sounds = Sounds {
        attack: chunk_store
            .load_wav("data\\wav\\_novice_attack.wav", asset_loader)
            .unwrap(),
        stun: chunk_store
            .load_wav("data\\wav\\_stun.wav", asset_loader)
            .unwrap(),
        heal: chunk_store
            .load_wav("data\\wav\\_heal_effect.wav", asset_loader)
            .unwrap(),
        firewall: chunk_store
            .load_wav("data\\wav\\effect\\ef_firewall.wav", asset_loader)
            .unwrap(),
    };
    return sounds;
}
