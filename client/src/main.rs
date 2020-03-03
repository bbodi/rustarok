//#![deny(
//    warnings,
//    anonymous_parameters,
//    unused_extern_crates,
//    unused_import_braces,
//    trivial_casts,
//    variant_size_differences,
//    trivial_numeric_casts,
//    unused_qualifications,
//    clippy::all
//)]

#[macro_use]
extern crate specs_derive;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::{BufRead, Read};
use std::net::TcpStream;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use log;
//use imgui::ImVec2;
use log::LevelFilter;
use rand::Rng;
use sdl2;
use specs;
use specs::prelude::*;
use specs::Builder;
use strum;

use rustarok_common::attack::{ApplyForceComponent, AreaAttackComponent, HpModificationRequest};
use rustarok_common::common::{
    measure_time, v2, EngineTime, LocalTime, ServerTime, SimulationTick, Vec2,
};
use rustarok_common::components::char::EntityTarget;
use rustarok_common::components::char::{
    create_common_player_entity, CharDir, CharOutlook, CollisionGroup, ControllerEntityId, JobId,
    LocalCharEntityId, LocalCharStateComp, ServerCharState, ServerEntityId, Sex,
    StaticCharDataComponent, Team,
};
use rustarok_common::components::job_ids::JobSpriteId;
use rustarok_common::config::CommonConfigs;
use rustarok_common::console::CommandArguments;
use rustarok_common::packets::from_server::{FromServerPacket, ServerEntityStateLocal};
use rustarok_common::packets::to_server::ToServerPacket;
use rustarok_common::packets::{NetworkTrafficEvent, PacketHandlerThread, SocketBuffer, SocketId};
use rustarok_common::systems::char_state_sys::CharacterStateUpdateSystem;

use crate::audio::sound_sys::{AudioCommandCollectorComponent, SoundSystem};
use crate::client::SimulationTime;
use crate::components::char::{
    create_client_entity, create_client_minion_entity, CharActionIndex, CharacterEntityBuilder,
    CharacterStateComponent, HasServerIdComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{
    CameraComponent, HumanInputComponent, LocalPlayerController, SkillKey,
};
use crate::components::skills::skills::{SkillManifestationComponent, Skills};
use crate::components::MinionComponent;
use crate::configs::AppConfig;
use crate::grf::asset_loader::GrfEntryLoader;
use crate::grf::database::AssetDatabase;
use crate::grf::SpriteResource;
use crate::my_gl::{Gl, MyGlEnum};
use crate::render::falcon_render_sys::FalconRenderSys;
use crate::render::opengl_render_sys::OpenGlRenderSystem;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::RenderDesktopClientSystem;
use crate::runtime_assets::audio::init_audio_and_load_sounds;
use crate::runtime_assets::ecs::create_ecs_world;
use crate::runtime_assets::effect::load_str_effects;
use crate::runtime_assets::graphic::{load_skill_icons, load_status_icons, load_texts};
use crate::runtime_assets::map::{load_map, MapRenderData, PhysicEngine};
use crate::systems::atk_calc::AttackSystem;
use crate::systems::camera_system::CameraSystem;
use crate::systems::console_system::{
    CommandDefinition, ConsoleComponent, ConsoleRenderSystem, ConsoleSystem,
};
use crate::systems::falcon_ai_sys::{FalconAiSystem, FalconComponent};
use crate::systems::frame_cleanup_system::FrameCleanupSystem;
use crate::systems::imgui_sys::{draw_imgui, ImguiData, ImguiSys};
use crate::systems::input_sys::InputConsumerSystem;
use crate::systems::input_to_next_action::{
    ClientIntentionToCharTargetSystem, InputToNextActionSystem,
};
use crate::systems::intention_sender_sys::{ClientCommandId, IntentionSenderSystem};
use crate::systems::minion_ai_sys::MinionAiSystem;
use crate::systems::next_action_applier_sys::{
    SavePreviousCharStateSystem, UpdateCharSpriteBasedOnStateSystem,
};
use crate::systems::phys::{FrictionSystem, PhysCollisionCollectorSystem};
use crate::systems::skill_sys::SkillSystem;
use crate::systems::snapshot_sys::{ServerAckResult, SnapshotStorage, SnapshotSystem};
use crate::systems::turret_ai_sys::TurretAiSystem;
use crate::systems::{
    CollisionsFromPrevFrame, RenderMatrices, Sprites, SystemFrameDurations, SystemVariables,
};
use crate::video::Video;

#[macro_use]
mod audio;
mod cam;
mod client;
mod configs;
mod consts;
mod cursor;
mod effect;
mod grf;
mod my_gl;
mod runtime_assets;
mod shaders;
// TODO2
//#[cfg(test)]
//mod tests;
mod video;

mod components;
mod render;
mod systems;

pub const SIMULATION_FREQ: usize = 31;
pub const SIMULATION_DURATION_MS: usize = 1000 / SIMULATION_FREQ;
pub const MAX_SECONDS_ALLOWED_FOR_SINGLE_SIMULATION_FRAME: f32 =
    SIMULATION_DURATION_MS as f32 / 1000.0;
pub const MAX_DURATION_ALLOWED_FOR_SINGLE_SIMULATION_FRAME: Duration =
    Duration::from_millis(SIMULATION_DURATION_MS as u64);

struct DelayedPacketReceiver {
    packets: Vec<(Instant, FromServerPacket)>,
    delay: Duration,
}

impl DelayedPacketReceiver {
    fn new(delay: Duration) -> DelayedPacketReceiver {
        DelayedPacketReceiver {
            packets: Vec::with_capacity(1024),
            delay,
        }
    }

    fn push(&mut self, packet: FromServerPacket) {
        self.packets.push((Instant::now(), packet));
    }

    fn get_packets(&mut self) -> Vec<FromServerPacket> {
        let tmp = std::mem::replace(&mut self.packets, vec![]);
        let (mut can_receive, other): (
            Vec<(Instant, FromServerPacket)>,
            Vec<(Instant, FromServerPacket)>,
        ) = tmp.into_iter().partition(|(received_at, packet)| {
            let now = Instant::now();
            (*received_at + self.delay) <= now
        });
        self.packets = other;
        return can_receive.into_iter().map(|it| it.1).collect();
    }
}

type OutPacketCollector = Vec<ToServerPacket>;

fn main() {
    log::info!("Loading config file config.toml");
    let config = AppConfig::new().expect("Could not load config file ('config.toml')");

    simple_logging::log_to_stderr(
        LevelFilter::from_str(&config.log_level)
            .expect("Unknown log level. Please set one of the following values for 'log_level' in 'config.toml': \"OFF\", \"ERROR\", \"WARN\", \"INFO\", \"DEBUG\", \"TRACE\"")
    );
    log::info!(">>> Loading GRF files");
    let (elapsed, asset_loader) = measure_time(|| {
        GrfEntryLoader::new(config.grf_paths.as_slice())
            .expect("Could not open grf files. Please configure them in 'config.toml'")
    });
    log::info!("<<< GRF loading: {}ms", elapsed.as_millis());

    log::info!("starting packet handler thread");
    let mut packet_handler_thread =
        PacketHandlerThread::<FromServerPacket, ToServerPacket>::start_thread(1);

    log::info!("Connecting to server");
    let server_socket = packet_handler_thread
        .handle_socket(TcpStream::connect(config.server_addr.clone()).unwrap());

    log::info!("sending welcome msg");
    packet_handler_thread.send(
        server_socket,
        ToServerPacket::Welcome {
            name: "sharp".to_owned(),
        },
    );

    log::info!("waiting for welcome response...");
    let (map_name, start_x, start_y, common_configs) = {
        let mut tmp_vec = Vec::with_capacity(64);
        'outer1: loop {
            packet_handler_thread.receive_into(&mut tmp_vec);
            let mut tmp_map_name = String::new();
            let mut tmp_start_x = 0.0;
            let mut tmp_start_y = 0.0;
            for (socket_id, packet) in tmp_vec.drain(..) {
                match packet {
                    NetworkTrafficEvent::Packet(FromServerPacket::Init {
                        map_name,
                        start_x,
                        start_y,
                    }) => {
                        tmp_map_name = map_name;
                        tmp_start_x = start_x;
                        tmp_start_y = start_y;
                        log::info!("answer received!!!");
                    }
                    NetworkTrafficEvent::Packet(FromServerPacket::Configs(configs)) => {
                        break 'outer1 (tmp_map_name, tmp_start_x, tmp_start_y, configs);
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    };

    let mut asset_db = AssetDatabase::new();

    let fov = 0.638;
    let _window_opened = false;
    let _cam_angle = -60.0;
    let render_matrices = RenderMatrices::new(fov, config.resolution_w, config.resolution_h);

    let sdl_context = sdl2::init().unwrap();
    let (mut video, gl, display_modes) =
        Video::init(&sdl_context, config.resolution_w, config.resolution_h);

    let mut physics_world = PhysicEngine::new();

    // dummy texture
    GrfEntryLoader::create_texture_from_surface(
        &gl,
        "dummy",
        asset_loader.backup_surface(),
        MyGlEnum::NEAREST,
        &mut asset_db,
    );

    log::info!(">>> Loading map");
    let map_render_data = load_map(
        &mut physics_world,
        &gl,
        &map_name,
        &asset_loader,
        &mut asset_db,
        config.load_models,
    );
    log::info!("<<< Loading map");

    let command_defs: HashMap<String, CommandDefinition> = ConsoleSystem::init_commands(
        get_all_effect_names(&asset_loader),
        get_all_map_names(&asset_loader),
        display_modes,
    );

    let mut ecs_world = create_ecs_world();

    let command_buffer = {
        let mut command_buffer = ConsoleCommandBuffer {
            commands: Vec::with_capacity(8),
        };
        let file = File::open("init.cmd").unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.unwrap();
            if line.starts_with("//") || line.trim().is_empty() {
                continue;
            }
            command_buffer.commands.push(line);
        }
        command_buffer
    };

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();
    log::info!(">>> load_str_effects");
    let (str_effects, str_effect_cache) = load_str_effects(&gl, &asset_loader, &mut asset_db);
    log::info!(">>> load_str_effects");
    let opengl_render_sys = OpenGlRenderSystem::new(gl.clone(), &ttf_context, str_effect_cache);
    log::info!(">>> Load sounds");
    let (maybe_sound_system, sounds) = init_audio_and_load_sounds(&sdl_context, &asset_loader);
    log::info!("<<< Load sounds");
    log::info!(">>> Populate SystemVariables");
    let sys_vars = SystemVariables::new(
        Sprites::new_for_test(),
        load_texts(&gl, &ttf_context, &mut asset_db),
        render_matrices,
        load_status_icons(&gl, &asset_loader, &mut asset_db),
        load_skill_icons(&gl, &asset_loader, &mut asset_db),
        str_effects,
        sounds,
        0.0, // fix dt, used only in tests
        config.resolution_w,
        config.resolution_h,
    );
    log::info!("<<< Populate SystemVariables");

    log::info!(">>> register systems");

    let mut ecs_client_dispatcher =
        { ClientEcs::new(Some(opengl_render_sys), maybe_sound_system, false) };
    log::info!("<<< register systems");
    log::info!(">>> add resources");
    // TODO: remove these
    ecs_world.insert(Vec::<HpModificationRequest>::with_capacity(128));
    ecs_world.insert(Vec::<AreaAttackComponent>::with_capacity(128));
    ecs_world.insert(Vec::<ApplyForceComponent>::with_capacity(128));

    ecs_world.insert(sys_vars);
    ecs_world.insert(common_configs);
    ecs_world.insert(ClientCommandId::new());
    ecs_world.insert(ImguiData::new(config.max_fps));
    if config.load_sprites {
        asset_loader.load_sprites(&gl, &mut asset_db);
    }
    ecs_world.insert(gl.clone());
    ecs_world.insert(map_render_data);
    ecs_world.insert(RenderCommandCollector::new());
    ecs_world.insert(command_buffer);
    ecs_world.insert(SimulationTime::new(SIMULATION_FREQ as usize));
    ecs_world.insert(SnapshotStorage::new());
    ecs_world.insert(OutPacketCollector::new());

    ecs_world.insert(asset_db);
    ecs_world.insert(server_socket);

    ecs_world.insert(CollisionsFromPrevFrame {
        collisions: HashMap::new(),
    });

    ecs_world.insert(physics_world);
    ecs_world.insert(SystemFrameDurations(HashMap::new()));
    ecs_world.insert(LocalPlayerController::new());
    ecs_world.insert(ConsoleComponent::new());

    ////////////////// SINGLETON Components
    {
        let mut human_player = HumanInputComponent::new();
        human_player.cast_mode = config.cast_mode;
        human_player.assign_skill(SkillKey::A, Skills::AttackMove);

        human_player.assign_skill(SkillKey::Q, Skills::FireWall);
        human_player.assign_skill(SkillKey::W, Skills::AbsorbShield);
        human_player.assign_skill(SkillKey::E, Skills::Heal);
        human_player.assign_skill(SkillKey::R, Skills::BrutalTestSkill);
        human_player.assign_skill(SkillKey::Y, Skills::Mounting);

        ecs_world.insert(RenderCommandCollector::new());
        ecs_world.insert(AudioCommandCollectorComponent::new());
        ecs_world.insert(human_player);
        // camera
        {
            let mut camera_component = CameraComponent::new();
            {
                let matrices = &ecs_world.read_resource::<SystemVariables>().matrices;
                camera_component.reset_y_and_angle(
                    &matrices.projection,
                    matrices.resolution_w,
                    matrices.resolution_h,
                );
            }
            ecs_world.insert(camera_component);
        }
    }

    let max_allowed_render_frame_duration = Duration::from_millis((1000 / config.max_fps) as u64);
    ecs_world.insert(config);

    ecs_world.maintain();
    log::info!("<<< add resources");

    let mut next_second: SystemTime = std::time::SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap();
    let mut next_minion_spawn = LocalTime::from(2.0);
    let mut fps_counter: usize = 0;
    let mut fps: usize;
    let mut incoming_packets_per_second: usize = 0;
    let mut incoming_bytes_per_second: usize = 0;
    let mut outgoing_bytes_per_second: usize = 0;
    let mut outgoing_packets_per_second: usize = 0;
    let mut fps_history: Vec<f32> = Vec::with_capacity(30);
    let mut system_frame_durations = SystemFrameDurations(HashMap::new());

    //////////////////////////////////////////////////
    // LOAD resources
    //////////////////////////////////////////////////
    asset_loader.no_more_requests();
    loop {
        if asset_loader.process_async_loading(
            &gl,
            &mut ecs_world.write_resource::<SystemVariables>(),
            &mut ecs_world.write_resource::<AssetDatabase>(),
            &mut ecs_world.write_resource::<MapRenderData>(),
        ) {
            // all requests have been processed
            break;
        } else {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    //////////////////////////////////////////////////

    let mut server_to_local_ids: HashMap<ServerEntityId, LocalCharEntityId> =
        HashMap::with_capacity(1024);

    console_print(&mut ecs_world, "Sync");
    {
        let mut avg_ping: u64 = 0;
        let mut tmp_vec = Vec::with_capacity(64);
        let mut server_tick = SimulationTick::new();
        for i in 0..1 {
            let sent_at = Instant::now();
            packet_handler_thread.send(server_socket, ToServerPacket::Ping);
            'outer2: loop {
                packet_handler_thread.receive_exact_into(&mut tmp_vec, 1);
                for (_socket_id, packet) in tmp_vec.drain(..) {
                    match packet {
                        NetworkTrafficEvent::Packet(FromServerPacket::Pong {
                            server_time: _server_time,
                            server_tick: tick,
                        }) => {
                            let ping = sent_at.elapsed().as_millis() as u64;
                            log::debug!("Pong arrived: ping: {:?}", ping);
                            if i == 0 {
                                avg_ping = ping;
                            } else {
                                avg_ping = (avg_ping + ping) / 2;
                            }
                            server_tick = tick;
                            break 'outer2;
                        }
                        _ => {}
                    }
                }
            }
        }
        ecs_world.insert(EngineTime::new(0));
        ecs_world.insert(server_tick);
        console_print(&mut ecs_world, &format!("avg ping: {}", avg_ping));

        packet_handler_thread.send(server_socket, ToServerPacket::ReadyForGame);
        // first ACK packet is for initializing our world state
        'outer3: loop {
            packet_handler_thread.receive_exact_into(&mut tmp_vec, 1);
            for (_socket_id, packet) in tmp_vec.drain(..) {
                match packet {
                    NetworkTrafficEvent::Packet(FromServerPacket::NewEntity {
                        id,
                        name,
                        team,
                        typ,
                        outlook,
                        job_id,
                        state,
                    }) => {
                        log::info!(">>> create player");
                        {
                            let username = ecs_world.read_resource::<AppConfig>().username.clone();
                            let desktop_client_char = create_client_entity(
                                &mut ecs_world,
                                username,
                                typ,
                                job_id,
                                state.pos,
                                team,
                                outlook,
                                id,
                            );

                            server_to_local_ids.insert(id, desktop_client_char);

                            ecs_world
                                .write_resource::<LocalPlayerController>()
                                .controller
                                .controlled_entity = Some(desktop_client_char);

                            // add falcon to it
                            let _falcon_id = ecs_world
                                .create_entity()
                                .with(FalconComponent::new(
                                    desktop_client_char,
                                    state.pos.x,
                                    state.pos.y,
                                ))
                                .with(SpriteRenderDescriptorComponent {
                                    action_index: CharActionIndex::Idle as usize,
                                    fps_multiplier: 1.0,
                                    animation_started: LocalTime::from(0.0),
                                    forced_duration: None,
                                    direction: CharDir::South,
                                    animation_ends_at: LocalTime::from(0.0),
                                })
                                .build();

                            ecs_world.maintain();
                        }
                        log::info!("<<< create player");
                        let mut snapshots = &mut ecs_world.write_resource::<SnapshotStorage>();
                        // we can mock the time here, does not count
                        snapshots.init(
                            id,
                            &LocalCharStateComp::server_to_local(
                                state,
                                LocalTime::from(0),
                                0,
                                &server_to_local_ids,
                            ),
                        );
                        break 'outer3;
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(100))
        }
    }

    console_print(&mut ecs_world, "Start");
    let mut tmp_vec = Vec::with_capacity(64);

    //    let mut packet_receiver = DelayedPacketReceiver::new(Duration::from_millis(0));
    let mut packet_receiver = Vec::with_capacity(128);

    packet_handler_thread.send(server_socket, ToServerPacket::Ping);
    let mut ping_sent = Instant::now();
    let mut avg_ping: usize = 0;
    let mut last_frame_duration = Duration::from_millis(0);

    let mut server_to_local_time_diff: i64 = 0;

    'running: loop {
        let now = ecs_world.read_resource::<EngineTime>().now();

        log::trace!(
            "START FRAME(now {:?}): simulation: ({}, {:?}), tail: {}",
            &ecs_world.read_resource::<EngineTime>().now(),
            &ecs_world
                .read_resource::<SimulationTime>()
                .can_simulation_run(),
            *ecs_world.read_resource::<SimulationTick>(),
            &ecs_world.read_resource::<SnapshotStorage>().get_tail()
        );
        let start = Instant::now();
        let simulation_frame: SimulationTick = *ecs_world.read_resource::<SimulationTick>();

        {
            packet_handler_thread.receive_into(&mut tmp_vec);
            for (_socket_id, packet) in tmp_vec.drain(..) {
                packet_receiver.push(packet);
            }
            incoming_packets_per_second += packet_receiver.len();
        }

        {
            let ack_result = {
                let mut ack_result = ServerAckResult::Ok;
                for packet in packet_receiver.drain(..) {
                    match packet {
                        NetworkTrafficEvent::IncomingTraffic { received_data_len } => {
                            incoming_bytes_per_second += received_data_len;
                        }
                        NetworkTrafficEvent::OutgoingTraffic { sent_data_len } => {
                            outgoing_bytes_per_second += sent_data_len;
                        }
                        NetworkTrafficEvent::LocalError(e) => { // TODO
                        }
                        NetworkTrafficEvent::Disconnected => { // TODO
                        }
                        NetworkTrafficEvent::Packet(p) => match p {
                            FromServerPacket::Init { .. } => panic!(),
                            FromServerPacket::Pong { server_time, .. } => {
                                let ping = ping_sent.elapsed().as_millis() as usize;
                                avg_ping = (avg_ping + ping) / 2;
                                let approximated_server_time =
                                    (server_time.0 + (ping as u32 / 2)) as i64;
                                let local_now =
                                    ecs_world.read_resource::<EngineTime>().now().as_millis()
                                        as i64;
                                server_to_local_time_diff = local_now - approximated_server_time;

                                packet_handler_thread.send(server_socket, ToServerPacket::Ping);
                                ping_sent = Instant::now();
                            }
                            FromServerPacket::Configs(configs) => {
                                log::info!("Configs has been updated by the server");
                                ecs_world
                                    .write_resource::<ConsoleComponent>()
                                    .print("Configs has been updated by the server");
                                dbg!(configs.stats.player.crusader.attributes.movement_speed);
                                *ecs_world.write_resource::<CommonConfigs>() = configs.clone();
                                for (state, static_info) in (
                                    &mut ecs_world.write_storage::<LocalCharStateComp>(),
                                    &ecs_world.read_storage::<StaticCharDataComponent>(),
                                )
                                    .join()
                                {
                                    state.recalc_attribs_based_on_statuses(
                                        static_info.job_id,
                                        &configs,
                                    );
                                }
                            }
                            FromServerPacket::Ack { cid, mut entries } => {
                                let snapshots = &mut ecs_world.write_resource::<SnapshotStorage>();

                                // TODO: replace in place
                                let entries = entries
                                    .into_iter()
                                    .map(|it| ServerEntityStateLocal {
                                        id: it.id,
                                        char_snapshot: LocalCharStateComp::server_to_local(
                                            it.char_snapshot,
                                            now,
                                            server_to_local_time_diff,
                                            &server_to_local_ids,
                                        ),
                                    })
                                    .collect();

                                ack_result = snapshots.ack_arrived(simulation_frame, cid, entries);
                                #[cfg(debug_assertions)]
                                {
                                    ecs_world
                                        .write_resource::<ImguiData>()
                                        .unacked_prediction_count(
                                            snapshots.get_unacked_prediction_count(),
                                        );
                                }
                            }
                            FromServerPacket::NewEntity {
                                id,
                                name,
                                team,
                                typ,
                                outlook,
                                job_id,
                                state,
                            } => {
                                log::info!(">>> create player");
                                {
                                    let char_entity_id = create_client_entity(
                                        &mut ecs_world,
                                        name,
                                        typ,
                                        job_id,
                                        state.pos,
                                        team,
                                        outlook,
                                        id,
                                    );

                                    server_to_local_ids.insert(id, char_entity_id);
                                }
                                ecs_world.maintain();

                                ecs_world
                                    .write_resource::<SnapshotStorage>()
                                    .add_predicting_entity(
                                        id,
                                        LocalCharStateComp::server_to_local(
                                            state,
                                            now,
                                            server_to_local_time_diff,
                                            &server_to_local_ids,
                                        ),
                                    );
                            }
                            FromServerPacket::PlayerDisconnected(disconnecting_entity_id) => {
                                let disconnecting_entity_local_id =
                                    server_to_local_ids[&disconnecting_entity_id];
                                log::info!(
                                    "{} has been disconnected",
                                    disconnecting_entity_local_id
                                );
                                ecs_world.delete_entity(disconnecting_entity_local_id.into());
                            }
                        },
                    }
                }
                ack_result
            };

            ecs_world
                .write_resource::<LocalPlayerController>()
                .had_been_rollbacked_in_this_frame = ack_result.is_rollback();

            match ack_result {
                ServerAckResult::ServerIsAheadOfClient {
                    server_state_updates,
                } => {
                    {
                        let snapshot_storage = &mut ecs_world.write_resource::<SnapshotStorage>();
                        snapshot_storage.reset_tail_index();
                        snapshot_storage.overwrite_states(&server_state_updates);
                    }
                    // the current tick's status is written into the snapshot storage in the above line,
                    // so this tick will in reality calculate the next tick's state, so increase
                    // the timer
                    load_all_last_acked_states_into_world(&mut ecs_world);

                    ecs_world.write_resource::<SimulationTick>().inc();
                    let timer = &mut ecs_world.write_resource::<SimulationTime>();
                    timer.force_simulation();
                }
                ServerAckResult::RemoteEntityCorrection => {
                    load_only_remote_last_acked_states_into_world(&mut ecs_world);
                }
                ServerAckResult::Rollback {
                    repredict_this_many_frames,
                } => {
                    log::trace!(
                        "Rollback {} frames from {}",
                        repredict_this_many_frames,
                        simulation_frame.as_u64() as usize - repredict_this_many_frames - 1,
                    );

                    load_all_last_acked_states_into_world(&mut ecs_world);
                    let original_timer: EngineTime =
                        (*ecs_world.read_resource::<EngineTime>().deref()).clone();
                    let reverted_timer = original_timer.reverted(
                        repredict_this_many_frames,
                        max_allowed_render_frame_duration,
                    );

                    ecs_world
                        .write_resource::<SimulationTick>()
                        .revert(repredict_this_many_frames);
                    ecs_world
                        .write_resource::<SimulationTime>()
                        .force_simulation();

                    *ecs_world.write_resource::<EngineTime>() = reverted_timer;

                    ecs_world
                        .write_resource::<SnapshotStorage>()
                        .reset_tail_index();
                    for i in 0..repredict_this_many_frames {
                        {
                            let snapshots = &mut ecs_world.write_resource::<SnapshotStorage>();
                            let (cid, intention) = snapshots.pop_intention();
                            snapshots.set_client_last_command_id(cid);
                            ecs_world
                                .write_resource::<LocalPlayerController>()
                                .controller
                                .intention = intention;
                        }
                        log::trace!("Rollback frame start");
                        //////////// DISPATCH ////////////////////////
                        ecs_client_dispatcher.run_only_predictions(&mut ecs_world);
                        log::trace!("Rollback frame end");
                        //////////////////////////////////////////////
                        ecs_world.write_resource::<SnapshotStorage>().tick();
                        ecs_world
                            .write_resource::<EngineTime>()
                            .tick(max_allowed_render_frame_duration);
                        ecs_world.write_resource::<SimulationTick>().inc();
                    }
                    {
                        log::trace!("after");
                        let snapshots = &ecs_world.read_resource::<SnapshotStorage>();
                        snapshots.print_snapshots_for(
                            0,
                            -2,
                            snapshots.get_unacked_prediction_count(),
                        );
                    }

                    *ecs_world.write_resource::<EngineTime>() = original_timer;

                    let snapshots = &ecs_world.read_resource::<SnapshotStorage>();
                    snapshots.print_snapshots_for(0, -2, repredict_this_many_frames);
                }
                ServerAckResult::Ok => {}
            }
        }

        #[cfg(debug_assertions)]
        {
            ecs_world.write_resource::<ImguiData>().rollback(
                ecs_world
                    .read_resource::<LocalPlayerController>()
                    .had_been_rollbacked_in_this_frame,
            );
        }

        let quit = !update_desktop_inputs(&mut video, &mut ecs_world);
        if quit {
            break 'running;
        }

        execute_console_commands(&mut ecs_world, &mut video, &command_defs);

        /////////////////////////////////////////////////////////////////////////////
        /////////////////////////////////////////////////////////////////////////////
        /////////////////////////////////////////////////////////////////////////////

        ecs_client_dispatcher.run_main_frame(&mut ecs_world, last_frame_duration, &command_defs);

        /////////////////////////////////////////////////////////////////////////////
        /////////////////////////////////////////////////////////////////////////////
        /////////////////////////////////////////////////////////////////////////////

        // TODO extract
        #[cfg(debug_assertions)]
        {
            video.imgui_sdl2.prepare_frame(
                video.imgui_context.io_mut(),
                &video.window,
                &video.event_pump.mouse_state(),
            );

            let delta_s = last_frame_duration.as_secs() as f32
                + last_frame_duration.subsec_nanos() as f32 / 1_000_000_000.0;
            video.imgui_context.io_mut().delta_time = delta_s;

            let mut ui = video.imgui_context.frame();
            draw_imgui(&mut ecs_world, &mut ui);

            video.imgui_sdl2.prepare_render(&ui, &video.window);
            video.imgui_renderer.render(ui);
        }

        video.gl_swap_window();

        outgoing_packets_per_second +=
            send_packets(&mut packet_handler_thread, server_socket, &mut ecs_world);

        // TODO: extract per_second_actions
        let now = std::time::SystemTime::now();
        if now >= next_second {
            fps = fps_counter;
            fps_history.push(fps as f32);
            if fps_history.len() > 30 {
                fps_history.remove(0);
            }

            {
                let benchmarks = &mut ecs_world.write_resource::<SystemFrameDurations>().0;
                system_frame_durations.0 = benchmarks.clone();
                benchmarks.clear();
            }

            fps_counter = 0;
            next_second = std::time::SystemTime::now()
                .checked_add(Duration::from_secs(1))
                .unwrap();

            video.set_title(&format!("Rustarok {} FPS", fps));
            log::debug!("FPS: {}, Ping: {}", fps, avg_ping);

            #[cfg(debug_assertions)]
            {
                let imgui_data = &mut ecs_world.write_resource::<ImguiData>();
                imgui_data.ping(avg_ping);
                imgui_data.fps(fps as usize);
                imgui_data.set_rollback();

                imgui_data.incoming_packets_per_second(incoming_packets_per_second);
                imgui_data.outgoing_packets_per_second(outgoing_packets_per_second);
                imgui_data.incoming_bytes_per_second(incoming_bytes_per_second);
                imgui_data.outgoing_bytes_per_second(outgoing_bytes_per_second);
                incoming_packets_per_second = 0;
                outgoing_packets_per_second = 0;
                incoming_bytes_per_second = 0;
                outgoing_bytes_per_second = 0;
            }
        }
        fps_counter += 1;

        // TODO2 minions
        //        let now = ecs_world.read_resource::<EngineTime>().now();
        //        if next_minion_spawn.has_already_passed(now)
        //            && ecs_world.read_resource::<CommonConfigs>().minions_enabled
        //        {
        //            next_minion_spawn = now.add_seconds(2.0);
        //            spawn_minions(&mut ecs_world)
        //        }

        let frame_duration = start.elapsed();
        if max_allowed_render_frame_duration > frame_duration {
            std::thread::sleep(max_allowed_render_frame_duration - frame_duration);
        } else {
            //            log::warn!(
            //                "Frame took too much time: {:?} > {:?}",
            //                frame_duration,
            //                max_allowed_render_frame_duration
            //            );
        }

        ecs_world.write_resource::<ImguiData>().simulation_duration(
            ecs_world
                .write_resource::<SimulationTime>()
                .get_time_between_simulations()
                .as_millis() as usize,
        );

        {
            let snapshots = &ecs_world.read_resource::<SnapshotStorage>();
            if snapshots.get_unacked_prediction_count() > 5 {
                //                timer.skip_next_simulation();
            }
        }

        last_frame_duration = frame_duration;
    }
}

fn send_packets(
    packet_handler_thread: &mut PacketHandlerThread<FromServerPacket, ToServerPacket>,
    server_socket: SocketId,
    ecs_world: &mut specs::World,
) -> usize {
    let mut to_server = ecs_world.write_resource::<Vec<ToServerPacket>>();
    let sent = to_server.len();
    for packet in to_server.drain(..) {
        packet_handler_thread.send(server_socket, packet);
    }
    return sent;
}

fn load_only_remote_last_acked_states_into_world(ecs_world: &mut World) {
    let snapshots = &ecs_world.read_resource::<SnapshotStorage>();
    let auth_storage = &mut ecs_world.write_storage::<LocalCharStateComp>();
    let server_id_storage = &ecs_world.read_storage::<HasServerIdComponent>();
    let entities = &ecs_world.entities();
    snapshots.load_last_acked_remote_entities_state_into_world(
        entities,
        auth_storage,
        server_id_storage,
        snapshots.get_last_acknowledged_index_for_server_entities(),
        Some(0),
    );
}

fn load_all_last_acked_states_into_world(ecs_world: &mut World) {
    let snapshots = &ecs_world.read_resource::<SnapshotStorage>();
    let auth_storage = &mut ecs_world.write_storage::<LocalCharStateComp>();
    let server_id_storage = &ecs_world.read_storage::<HasServerIdComponent>();
    let entities = &ecs_world.entities();
    snapshots.load_last_acked_remote_entities_state_into_world(
        entities,
        auth_storage,
        server_id_storage,
        snapshots.get_last_acknowledged_index(),
        None,
    );
}

pub fn console_print(ecs_world: &mut specs::World, text: &str) {
    log::debug!("{}", text);
    ecs_world.write_resource::<ConsoleComponent>().print(text);
}

struct ClientEcs<'a, 'b> {
    simulation_dispatcher: Dispatcher<'a, 'b>,
    render_dispatcher: Dispatcher<'a, 'b>,
    prediction_dispatcher: Dispatcher<'a, 'b>,
    input_consumer_sys: InputConsumerSystem,
    input_to_next_action_sys: InputToNextActionSystem,
}

impl<'a, 'b> ClientEcs<'a, 'b> {
    pub fn new(
        opengl_render_sys: Option<OpenGlRenderSystem<'b, 'b>>,
        maybe_sound_system: Option<SoundSystem>,
        for_test: bool,
    ) -> ClientEcs<'a, 'b> {
        ClientEcs {
            simulation_dispatcher: ClientEcs::create_with_simulation_systems(for_test),
            render_dispatcher: ClientEcs::create_without_simulation_systems(
                opengl_render_sys,
                maybe_sound_system,
            ),
            prediction_dispatcher: ClientEcs::create_dispatcher_for_predictions(),
            input_consumer_sys: InputConsumerSystem,
            input_to_next_action_sys: InputToNextActionSystem::new(),
        }
    }

    pub fn run_main_frame(
        &mut self,
        ecs_world: &mut specs::World,
        dt: Duration,
        command_defs: &HashMap<String, CommandDefinition>,
    ) {
        if ecs_world
            .read_resource::<SimulationTime>()
            .can_simulation_run()
        {
            self.input_consumer_sys.run(
                &mut ecs_world.write_resource(),
                &mut ecs_world.write_resource(),
                &mut ecs_world.write_resource(),
                &ecs_world.read_resource::<SystemVariables>().matrices,
            );

            ConsoleSystem::run(
                command_defs,
                &ecs_world.read_resource::<HumanInputComponent>(),
                &ecs_world.read_storage::<StaticCharDataComponent>(),
                &mut ecs_world.write_resource::<ConsoleComponent>(),
                &ecs_world.read_resource::<SystemVariables>(),
                &ecs_world.read_resource::<EngineTime>(),
            );

            self.input_to_next_action_sys.run(
                &ecs_world.read_resource(),
                ecs_world.read_storage(),
                ecs_world.read_storage(),
                &mut ecs_world.write_resource(),
                &mut ecs_world.write_resource(),
                &ecs_world.read_resource(),
                &ecs_world.read_resource(),
                *ecs_world.read_resource(),
                &ecs_world.read_resource::<MapRenderData>(),
            );

            self.simulation_dispatcher.dispatch(ecs_world);
            execute_finished_skill_castings(ecs_world);
            ecs_world.write_resource::<SnapshotStorage>().tick();
            ecs_world.write_resource::<SimulationTick>().inc();
        }

        ConsoleRenderSystem::run(
            &ecs_world.read_resource::<ConsoleComponent>(),
            &mut ecs_world.write_resource(),
            &ecs_world.read_resource(),
            command_defs,
        );
        self.render_dispatcher.dispatch(ecs_world);

        ecs_world.maintain();

        ecs_world
            .write_resource::<SimulationTime>()
            .render_frame_end(Instant::now());
        ecs_world.write_resource::<EngineTime>().tick(dt);
    }

    pub fn run_only_predictions(&mut self, ecs_world: &mut specs::World) {
        self.prediction_dispatcher.dispatch(ecs_world);
    }

    fn create_without_simulation_systems(
        opengl_render_sys: Option<OpenGlRenderSystem<'b, 'b>>,
        maybe_sound_system: Option<SoundSystem>,
    ) -> Dispatcher<'a, 'b> {
        let ecs_dispatcher = {
            let mut ecs_dispatcher_builder = specs::DispatcherBuilder::new();

            ecs_dispatcher_builder =
                ecs_dispatcher_builder.with(CameraSystem, "camera_system", &[]);

            ecs_dispatcher_builder = ecs_dispatcher_builder
                .with_thread_local(RenderDesktopClientSystem::new())
                .with_thread_local(FalconRenderSys)
                .with_thread_local(opengl_render_sys.unwrap());
            if let Some(sound_system) = maybe_sound_system {
                ecs_dispatcher_builder = ecs_dispatcher_builder.with_thread_local(sound_system);
            }
            // only simulation frames cretaes new graphics/sounds, so it is enough to clean up here
            ecs_dispatcher_builder = ecs_dispatcher_builder.with_thread_local(FrameCleanupSystem);
            ecs_dispatcher_builder.build()
        };
        ecs_dispatcher
    }

    fn create_with_simulation_systems(for_test: bool) -> Dispatcher<'a, 'b> {
        let ecs_dispatcher = {
            let mut ecs_dispatcher_builder = specs::DispatcherBuilder::new();
            if !for_test {
                ecs_dispatcher_builder = ecs_dispatcher_builder.with(
                    IntentionSenderSystem::new(SIMULATION_FREQ),
                    "intention_sender",
                    &[],
                );
            }
            ecs_dispatcher_builder = ecs_dispatcher_builder.with(
                ClientIntentionToCharTargetSystem,
                "client_intention_to_char_target_system",
                &[],
            );
            ecs_dispatcher_builder =
                ecs_dispatcher_builder.with(FrictionSystem, "friction_sys", &[]);
            //            .with(MinionAiSystem, "minion_ai_sys", &[])
            //                .with(TurretAiSystem, "turret_ai_sys", &[])
            //                .with(FalconAiSystem, "falcon_ai_sys", &[])
            if !for_test {
                ecs_dispatcher_builder.add(
                    UpdateCharSpriteBasedOnStateSystem,
                    "UpdateCharSpriteBasedOnStateSystem",
                    &["client_intention_to_char_target_system"],
                );
                ecs_dispatcher_builder.add(
                    SavePreviousCharStateSystem,
                    "SavePreviousCharStateSystem",
                    &["UpdateCharSpriteBasedOnStateSystem"],
                );
            } else {
                ecs_dispatcher_builder.add(
                    SavePreviousCharStateSystem,
                    "SavePreviousCharStateSystem",
                    &["client_intention_to_char_target_system"],
                );
            }
            ecs_dispatcher_builder = ecs_dispatcher_builder
                .with(
                    CharacterStateUpdateSystem,
                    "char_state_update",
                    &["client_intention_to_char_target_system"],
                )
                .with(
                    PhysCollisionCollectorSystem,
                    "collision_collector",
                    &["char_state_update"],
                )
                .with(SkillSystem, "skill_sys", &["collision_collector"])
                .with(AttackSystem::new(), "attack_sys", &["collision_collector"])
                .with(SnapshotSystem::new(), "snapshot_sys", &["attack_sys"]);

            ecs_dispatcher_builder.build()
        };
        ecs_dispatcher
    }

    fn create_dispatcher_for_predictions() -> Dispatcher<'a, 'b> {
        return specs::DispatcherBuilder::new()
            .with_thread_local(ClientIntentionToCharTargetSystem)
            .with_thread_local(CharacterStateUpdateSystem)
            .with_thread_local(SnapshotSystem::new())
            .build();
    }
}

fn update_desktop_inputs(video: &mut Video, ecs_world: &mut World) -> bool {
    let inputs = &mut ecs_world.write_resource::<HumanInputComponent>();

    for event in video.event_pump.poll_iter() {
        video
            .imgui_sdl2
            .handle_event(&mut video.imgui_context, &event);
        if video.imgui_sdl2.ignore_event(&event) {
            continue;
        }
        match event {
            sdl2::event::Event::Quit { .. } => {
                return false;
            }
            sdl2::event::Event::Window {
                timestamp,
                window_id,
                win_event,
            } => match win_event {
                sdl2::event::WindowEvent::Resized(w, h) => {
                    let gl = &ecs_world.read_resource::<Gl>();
                    gl.viewport(0, 0, w, h);
                    ecs_world.write_resource::<SystemVariables>().matrices =
                        RenderMatrices::new(0.638, w as u32, h as u32);
                }
                _ => {}
            },
            _ => {
                inputs.inputs.push(event);
            }
        }
    }
    return true;
}

fn spawn_minions(ecs_world: &mut World) -> () {
    {
        let entity_id = create_client_minion_entity(
            ecs_world,
            v2(
                MinionAiSystem::CHECKPOINTS[0][0] as f32,
                MinionAiSystem::CHECKPOINTS[0][1] as f32,
            ),
            Team::Right,
        );

        let mut storage = ecs_world.write_storage();
        storage
            .insert(entity_id.into(), MinionComponent { fountain_up: false })
            .unwrap();
    }
    {
        let entity_id = create_client_minion_entity(
            ecs_world,
            v2(
                MinionAiSystem::CHECKPOINTS[5][0] as f32,
                MinionAiSystem::CHECKPOINTS[5][1] as f32,
            ),
            Team::Left,
        );
        let mut storage = ecs_world.write_storage();
        storage
            .insert(entity_id.into(), MinionComponent { fountain_up: false })
            .unwrap();
    }
}

pub struct ConsoleCommandBuffer {
    commands: Vec<String>,
}

fn execute_console_commands(
    ecs_world: &mut World,
    video: &mut Video,
    command_defs: &HashMap<String, CommandDefinition>,
) {
    {
        let console_args = {
            let console = &mut ecs_world.write_resource::<ConsoleComponent>();
            std::mem::replace(&mut console.command_to_execute, None)
        };
        if let Some(cmd) = console_args {
            execute_console_command(cmd, ecs_world, video, command_defs);
        }
    }

    // run commands from key_bindings
    {
        let commands = {
            let mut command_buffer = ecs_world.write_resource::<ConsoleCommandBuffer>();
            std::mem::replace(&mut command_buffer.commands, Vec::with_capacity(8))
        };
        for command in commands.into_iter() {
            let cmd = CommandArguments::new(&command);
            execute_console_command(cmd, ecs_world, video, command_defs);
        }
    }

    ecs_world.maintain();
}

fn execute_console_command(
    cmd: CommandArguments,
    ecs_world: &mut World,
    video: &mut Video,
    command_defs: &HashMap<String, CommandDefinition>,
) {
    let char_entity_id = ecs_world
        .read_resource::<LocalPlayerController>()
        .controller
        .controlled_entity;

    log::debug!("Execute command: {:?}", cmd);
    let command_def = &command_defs[cmd.get_command_name().unwrap()];
    if let Err(e) = (command_def.action)(char_entity_id, cmd, ecs_world, video) {
        log::error!("Console error: {}", e);
        ecs_world.write_resource::<ConsoleComponent>().error(&e);
    }
}

fn execute_finished_skill_castings(ecs_world: &mut World) {
    // TODO: avoid allocating new vec
    let finished_casts = std::mem::replace(
        &mut ecs_world
            .write_resource::<SystemVariables>()
            .just_finished_skill_casts,
        Vec::with_capacity(128),
    );
    for finished_cast in &finished_casts {
        let manifestation = finished_cast
            .skill
            .get_definition()
            .finish_cast(&finished_cast, ecs_world);
        if let Some(manifestation) = manifestation {
            let skill_entity_id = ecs_world.create_entity().build();
            ecs_world.read_resource::<LazyUpdate>().insert(
                skill_entity_id,
                SkillManifestationComponent::new(skill_entity_id, manifestation),
            );
        }
    }
}

fn get_all_effect_names(asset_loader: &GrfEntryLoader) -> Vec<String> {
    let all_str_names = asset_loader
        .asset_loader
        .read_dir("data\\texture\\effect")
        .into_iter()
        .filter(|file_name| file_name.ends_with("str"))
        .map(|mut file_name| {
            file_name.drain(.."data\\texture\\effect\\".len()); // remove dir from the beginning
            let len = file_name.len();
            file_name.truncate(len - 4); // and extension from the end
            file_name
        })
        .collect::<Vec<String>>();
    all_str_names
}

fn get_all_map_names(asset_loader: &GrfEntryLoader) -> Vec<String> {
    let all_map_names = asset_loader
        .asset_loader
        .read_dir("data")
        .into_iter()
        .filter(|file_name| file_name.ends_with("rsw"))
        .map(|mut file_name| {
            file_name.drain(..5); // remove "data\\" from the begining
            let len = file_name.len();
            file_name.truncate(len - 4); // and extension from the end
            file_name
        })
        .collect::<Vec<String>>();
    all_map_names
}
