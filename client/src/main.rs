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

use crossbeam_channel;
use log;
use notify;
use sdl2;
use specs;
#[macro_use]
extern crate specs_derive;
use strum;

use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};

//use imgui::ImVec2;
use log::LevelFilter;
use rand::Rng;
use specs::prelude::*;
use specs::Builder;
use specs::Join;

use crate::audio::sound_sys::SoundSystem;
use crate::components::char::{
    CharActionIndex, CharacterEntityBuilder, CharacterStateComponent, DebugServerAckComponent,
    HasServerIdComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::HumanInputComponent;
use crate::components::skills::skills::SkillManifestationComponent;
use crate::components::MinionComponent;
use crate::configs::{AppConfig, DevConfig};
use crate::grf::asset_loader::GrfEntryLoader;
use crate::grf::database::AssetDatabase;
use crate::grf::SpriteResource;
use crate::my_gl::MyGlEnum;
use crate::notify::Watcher;
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
    CommandArguments, CommandDefinition, ConsoleComponent, ConsoleSystem,
};
use crate::systems::falcon_ai_sys::{FalconAiSystem, FalconComponent};
use crate::systems::frame_cleanup_system::FrameCleanupSystem;
use crate::systems::frame_client_end_system::FrameClientEndSystem;
use crate::systems::input_sys::InputConsumerSystem;
use crate::systems::input_to_next_action::InputToNextActionSystem;
use crate::systems::intention_sender_sys::IntentionSenderSystem;
use crate::systems::minion_ai_sys::MinionAiSystem;
use crate::systems::next_action_applier_sys::{
    SavePreviousCharStateSystem, UpdateCharSpriteBasedOnStateSystem,
};
use crate::systems::phys::{FrictionSystem, PhysCollisionCollectorSystem};
use crate::systems::skill_sys::SkillSystem;
use crate::systems::snapshot_sys::{
    DebugServerAckComponentFillerSystem, GameSnapshots, ServerAckResult, SnapshotSystem,
};
use crate::systems::turret_ai_sys::TurretAiSystem;
use crate::systems::{
    CollisionsFromPrevFrame, RenderMatrices, Sprites, SystemFrameDurations, SystemVariables,
};
use crate::video::Video;
use rustarok_common::common::{
    measure_time, v2, ElapsedTime, EngineTime, Vec2, MAX_DURATION_ALLOWED_FOR_SINGLE_FRAME,
};
use rustarok_common::components::char::{
    AuthorizedCharStateComponent, CharDir, CharEntityId, CharOutlook, CollisionGroup,
    ControllerEntityId, JobId, ServerEntityId, Sex, Team,
};
use rustarok_common::components::controller::ControllerComponent;
use rustarok_common::components::job_ids::JobSpriteId;
use rustarok_common::packets::from_server::FromServerPacket;
use rustarok_common::packets::to_server::ToServerPacket;
use rustarok_common::packets::{PacketHandlerThread, SocketBuffer};
use rustarok_common::systems::char_state_sys::CharacterStateUpdateSystem;
use rustarok_common::systems::intention_applier::NextActionApplierSystem;
use std::fs::File;
use std::io::BufReader;
use std::io::{BufRead, Read};
use std::net::TcpStream;
use std::ops::{Add, Deref, DerefMut};

#[macro_use]
mod audio;
mod cam;
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

#[macro_use]
mod components;
mod render;
mod systems;

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

fn main() {
    log::info!("Loading config file config.toml");
    let config = AppConfig::new().expect("Could not load config file ('config.toml')");
    let (mut runtime_conf_watcher_rx, mut watcher) = {
        let (tx, runtime_conf_watcher_rx) = crossbeam_channel::unbounded();
        let mut watcher = notify::watcher(tx.clone(), Duration::from_secs(2)).unwrap();
        watcher
            .watch("config-runtime.toml", notify::RecursiveMode::NonRecursive)
            .unwrap();
        (runtime_conf_watcher_rx, watcher)
    };

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
    let server_socket =
        packet_handler_thread.handle_socket(TcpStream::connect(config.server_addr).unwrap());

    log::info!("sending welcome msg");
    packet_handler_thread.send(
        server_socket,
        ToServerPacket::Welcome {
            name: "sharp".to_owned(),
        },
    );

    log::info!("waiting for welcome response...");
    let (map_name, start_x, start_y) = {
        'outer1: loop {
            let mut tmp_vec = Vec::with_capacity(64);
            packet_handler_thread.receive_into(&mut tmp_vec);
            for (socket_id, packet) in tmp_vec {
                match packet {
                    FromServerPacket::Init {
                        map_name,
                        start_x,
                        start_y,
                    } => {
                        log::info!("answer received!!!");
                        break 'outer1 (map_name, start_x, start_y);
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
    let mut ecs_client_dispatcher = {
        let console_sys = ConsoleSystem::new(&command_defs);
        register_systems(
            Some(opengl_render_sys),
            maybe_sound_system,
            Some(console_sys),
            false,
        )
    };
    let mut prediction_dispatcher = create_dispatcher_for_predictions();
    //    let mut ecs_server_dispatcher = register_server_systems();
    log::info!("<<< register systems");
    log::info!(">>> add resources");
    ecs_world.add_resource(sys_vars);
    if config.load_sprites {
        asset_loader.load_sprites(&gl, &mut asset_db);
    }
    ecs_world.add_resource(gl.clone());
    ecs_world.add_resource(map_render_data);
    ecs_world.add_resource(DevConfig::new().unwrap());
    ecs_world.add_resource(RenderCommandCollector::new());
    ecs_world.add_resource(command_buffer);
    ecs_world.add_resource(EngineTime::new());
    ecs_world.add_resource(GameSnapshots::new());
    ecs_world.add_resource(Vec::<ToServerPacket>::new());

    ecs_world.add_resource(asset_db);
    ecs_world.add_resource(server_socket);

    ecs_world.add_resource(CollisionsFromPrevFrame {
        collisions: HashMap::new(),
    });

    ecs_world.add_resource(physics_world);
    ecs_world.add_resource(SystemFrameDurations(HashMap::new()));
    log::info!("<<< add resources");

    let mut next_second: SystemTime = std::time::SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap();
    let mut next_minion_spawn = ElapsedTime(2.0);
    let mut fps_counter: u64 = 0;
    let mut fps: u64;
    let mut fps_history: Vec<f32> = Vec::with_capacity(30);
    let mut system_frame_durations = SystemFrameDurations(HashMap::new());

    let desktop_client_controller = ControllerEntityId::new(ecs_world.create_entity().build());
    ecs_world
        .read_resource::<LazyUpdate>()
        .insert(desktop_client_controller.into(), ConsoleComponent::new());
    ecs_world.maintain();
    console_print(&mut ecs_world, "Sync", desktop_client_controller);
    {
        let mut avg_ping = 0;
        let mut tmp_vec = Vec::with_capacity(64);
        let mut server_tick = 0;
        for i in 0..1 {
            let sent_at = Instant::now();
            packet_handler_thread.send(server_socket, ToServerPacket::Ping);
            'outer2: loop {
                packet_handler_thread.receive_exact_into(&mut tmp_vec, 1);
                for (_socket_id, packet) in tmp_vec.drain(..) {
                    match packet {
                        FromServerPacket::Pong { server_tick: tick } => {
                            let ping = sent_at.elapsed();
                            log::debug!("Pong arrived: ping: {:?}", ping);
                            if i == 0 {
                                avg_ping = ping.as_millis();
                            } else {
                                avg_ping = (avg_ping + ping.as_millis()) / 2;
                            }
                            server_tick = tick;
                            break 'outer2;
                        }
                        _ => {}
                    }
                }
            }
        }
        console_print(
            &mut ecs_world,
            &format!("avg ping: {}", avg_ping),
            desktop_client_controller,
        );

        packet_handler_thread.send(server_socket, ToServerPacket::ReadyForGame);
        // first ACK packet is for initializing out world state
        'outer3: loop {
            packet_handler_thread.receive_exact_into(&mut tmp_vec, 1);
            for (_socket_id, packet) in tmp_vec.drain(..) {
                match packet {
                    FromServerPacket::Ack {
                        cid: _cid,
                        ack_tick,
                        entries,
                    } => {
                        let entry = &entries[0];
                        log::info!(">>> create player");
                        {
                            let desktop_client_char =
                                CharEntityId::from(ecs_world.create_entity().build());
                            components::char::attach_human_player_components(
                                "sharp",
                                desktop_client_char,
                                desktop_client_controller,
                                &ecs_world.read_resource::<LazyUpdate>(),
                                &mut ecs_world.write_resource::<PhysicEngine>(),
                                ecs_world
                                    .read_resource::<SystemVariables>()
                                    .matrices
                                    .projection,
                                v2(start_x, start_y),
                                Sex::Male,
                                JobId::CRUSADER,
                                1,
                                Team::Right,
                                &ecs_world.read_resource::<DevConfig>(),
                                config.resolution_w,
                                config.resolution_h,
                                entry.id,
                            );

                            // add falcon to it
                            let start_x = start_x;
                            let start_y = start_y;
                            let _falcon_id = ecs_world
                                .create_entity()
                                .with(FalconComponent::new(desktop_client_char, start_x, start_y))
                                .with(SpriteRenderDescriptorComponent {
                                    action_index: CharActionIndex::Idle as usize,
                                    fps_multiplier: 1.0,
                                    animation_started: ElapsedTime(0.0),
                                    forced_duration: None,
                                    direction: CharDir::South,
                                    animation_ends_at: ElapsedTime(0.0),
                                })
                                .build();

                            ecs_world.maintain();
                        }
                        log::info!("<<< create player");
                        let mut snapshots = &mut ecs_world.write_resource::<GameSnapshots>();
                        snapshots.init(entry);
                        ecs_world.write_resource::<EngineTime>().tick = ack_tick + 1;
                        break 'outer3;
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(100))
        }
    }

    console_print(&mut ecs_world, "Start", desktop_client_controller);
    let mut tmp_vec = Vec::with_capacity(64);

    let mut packet_receiver = DelayedPacketReceiver::new(Duration::from_millis(0));
    let mut client_speed_increaser = Duration::from_millis(0);
    let mut server_to_local_ids: HashMap<ServerEntityId, CharEntityId> =
        HashMap::with_capacity(1024);
    'running: loop {
        let start = Instant::now();
        let tick = ecs_world.read_resource::<EngineTime>().tick;

        {
            packet_handler_thread.receive_into(&mut tmp_vec);
            for (_socket_id, packet) in tmp_vec.drain(..) {
                packet_receiver.push(packet);
            }
        }

        {
            let mut server_is_ahead_of_client = false;
            let (ack_result, acked_tick) = {
                let mut ack_result = ServerAckResult::Ok;
                let mut tmp_ack_tick = 0;
                for packet in packet_receiver.get_packets().drain(..) {
                    match packet {
                        FromServerPacket::LocalError(_) => {}
                        FromServerPacket::Init { .. } => panic!(),
                        FromServerPacket::Pong { .. } => panic!(),
                        FromServerPacket::Ack {
                            cid,
                            ack_tick,
                            mut entries,
                        } => {
                            if server_is_ahead_of_client {
                                continue;
                            }
                            if ack_tick >= tick {
                                server_is_ahead_of_client = true;
                                continue;
                            }
                            let snapshots = &mut ecs_world.write_resource::<GameSnapshots>();
                            let debug_ack_storage =
                                &mut ecs_world.write_storage::<DebugServerAckComponent>();
                            tmp_ack_tick = ack_tick;

                            ack_result = snapshots.ack_arrived(tick, cid, ack_tick, &entries);

                            // SHIT asd
                            let auth_state_storage =
                                &mut ecs_world.write_storage::<AuthorizedCharStateComponent>();
                            for server_entity_state in entries.drain(1..) {
                                let local_id =
                                    server_to_local_ids.get(&server_entity_state.id).unwrap();
                                let char_state =
                                    auth_state_storage.get_mut((*local_id).into()).unwrap();
                                char_state.target = server_entity_state.char_snapshot.state.target;
                            }
                        }
                        FromServerPacket::NewEntity {
                            id,
                            name,
                            team,
                            typ,
                            outlook,
                            job_id,
                            max_hp,
                            state,
                        } => {
                            log::info!(">>> create player");
                            {
                                let char_entity_id =
                                    CharEntityId::from(ecs_world.create_entity().build());
                                let updater = &ecs_world.read_resource::<LazyUpdate>();
                                let dev_configs = &ecs_world.read_resource::<DevConfig>();
                                CharacterEntityBuilder::new(char_entity_id, &name)
                                    .insert_sprite_render_descr_component(updater)
                                    .server_authorized(updater, id)
                                    .physics(
                                        state.state.pos(),
                                        &mut ecs_world.write_resource::<PhysicEngine>(),
                                        |builder| {
                                            builder
                                                .collision_group(team.get_collision_group())
                                                .circle(1.0)
                                        },
                                    )
                                    .char_state(updater, dev_configs, state.state.pos(), |ch| {
                                        ch.outlook(outlook).job_id(job_id).team(team)
                                    });
                                server_to_local_ids.insert(id, char_entity_id);
                            }
                            ecs_world.maintain();

                            ecs_world
                                .write_resource::<GameSnapshots>()
                                .add_predicting_entity(id, state.clone().state);
                        }
                    }
                }
                (ack_result, tmp_ack_tick)
            };

            if server_is_ahead_of_client {
                log::debug!("ServerIsAheadOfClient: {} -> {}", tick, acked_tick);
                client_speed_increaser += Duration::from_millis(1);
            }
            match ack_result {
                ServerAckResult::Rollback {
                    repredict_this_many_frames,
                } => {
                    log::debug!("Rollback: {} <-- {}", acked_tick, tick);
                    load_last_acked_state_into_world(&mut ecs_world, desktop_client_controller);
                    let timer: EngineTime =
                        (*ecs_world.read_resource::<EngineTime>().deref()).clone();
                    let reverted_timer = timer.reverted_to(acked_tick + 1);
                    *ecs_world.write_resource::<EngineTime>() = reverted_timer;

                    ecs_world
                        .write_resource::<GameSnapshots>()
                        .reset_tail_index();
                    for i in 0..repredict_this_many_frames {
                        {
                            let snapshots = &mut ecs_world.write_resource::<GameSnapshots>();
                            let (cid, intention) = snapshots.pop_intention();
                            snapshots.set_client_last_command_id(dbg!(cid));
                            ecs_world
                                .write_storage::<ControllerComponent>()
                                .get_mut(desktop_client_controller.into())
                                .unwrap()
                                .intention = intention;
                        }
                        prediction_dispatcher.dispatch(&mut ecs_world.res);
                        ecs_world
                            .write_resource::<EngineTime>()
                            .update_timers_for_prediction();
                    }
                    *ecs_world.write_resource::<EngineTime>() = timer;
                    let snapshots = &ecs_world.read_resource::<GameSnapshots>();
                    snapshots.print_snapshots_for(0, -2, repredict_this_many_frames);
                }
                ServerAckResult::Ok => {
                    if acked_tick > 0
                        && tick > acked_tick + 10
                        && client_speed_increaser > Duration::from_millis(0)
                    {
                        log::info!("Slow down (acked_tick: {}, client: {})", acked_tick, tick);
                        client_speed_increaser -= Duration::from_millis(1);
                    } else if tick < acked_tick + 5 {
                        log::info!("Speed up (acked_tick: {}, client: {})", acked_tick, tick);
                        client_speed_increaser += Duration::from_millis(1);
                    }
                }
            }
        }

        asset_loader.process_async_loading(
            &gl,
            &mut ecs_world.write_resource::<SystemVariables>(),
            &mut ecs_world.write_resource::<AssetDatabase>(),
            &mut ecs_world.write_resource::<MapRenderData>(),
        );

        let quit = !update_desktop_inputs(&mut video, &mut ecs_world, desktop_client_controller);
        if quit {
            break 'running;
        }

        execute_console_commands(
            &command_defs,
            &mut ecs_world,
            desktop_client_controller,
            &mut video,
        );
        //        ecs_server_dispatcher.dispatch(&mut ecs_world.res);
        run_main_frame(&mut ecs_world, &mut ecs_client_dispatcher);

        video.gl_swap_window();

        {
            let mut to_server = ecs_world.write_resource::<Vec<ToServerPacket>>();
            for packet in to_server.drain(..) {
                packet_handler_thread.send(server_socket, packet);
            }
        }

        std::thread::sleep(Duration::from_millis(
            ecs_world.read_resource::<DevConfig>().sleep_ms,
        ));
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
            log::debug!("FPS: {}", fps);
        }
        fps_counter += 1;

        let now = ecs_world.read_resource::<EngineTime>().now();
        if next_minion_spawn.has_already_passed(now)
            && ecs_world.read_resource::<DevConfig>().minions_enabled
        {
            next_minion_spawn = now.add_seconds(2.0);
            spawn_minions(&mut ecs_world)
        }

        // runtime configs
        let ret = reload_configs_if_changed(runtime_conf_watcher_rx, watcher, &mut ecs_world);
        runtime_conf_watcher_rx = ret.0;
        watcher = ret.1;

        let frame_duration = start.elapsed();
        if frame_duration + client_speed_increaser < MAX_DURATION_ALLOWED_FOR_SINGLE_FRAME {
            let to_sleep =
                MAX_DURATION_ALLOWED_FOR_SINGLE_FRAME - (frame_duration + client_speed_increaser);
            std::thread::sleep(to_sleep);
        }
        //        std::thread::sleep(Duration::from_millis(1000));
        let mut timer = ecs_world.write_resource::<EngineTime>();
        timer.update_timers(MAX_DURATION_ALLOWED_FOR_SINGLE_FRAME, Instant::now());
    }
}

fn load_last_acked_state_into_world(
    ecs_world: &mut World,
    desktop_client_controller: ControllerEntityId,
) {
    let snapshots = &ecs_world.read_resource::<GameSnapshots>();
    let auth_storage = &mut ecs_world.write_storage::<AuthorizedCharStateComponent>();
    let server_id_storage = &ecs_world.read_storage::<HasServerIdComponent>();
    let ack_debug_storage = &mut ecs_world.write_storage::<DebugServerAckComponent>();
    let entities = &ecs_world.entities();
    snapshots.load_last_acked_state_into_world(
        entities,
        auth_storage,
        server_id_storage,
        ack_debug_storage,
    );
}

pub fn run_main_frame(mut ecs_world: &mut World, ecs_dispatcher: &mut Dispatcher) {
    ecs_dispatcher.dispatch(&mut ecs_world.res);
    execute_finished_skill_castings(&mut ecs_world);
    ecs_world.maintain();
}

//fn register_server_systems<'a, 'b>() -> Dispatcher<'a, 'b> {
//    let ecs_dispatcher = {
//        let mut ecs_dispatcher_builder = specs::DispatcherBuilder::new();
//        ecs_dispatcher_builder = ecs_dispatcher_builder.with(FrictionSystem, "friction_sys", &[]);
//        ecs_dispatcher_builder = ecs_dispatcher_builder
//            .with(MinionAiSystem, "minion_ai_sys", &[])
//            .with(TurretAiSystem, "turret_ai_sys", &[])
//            .with(FalconAiSystem, "falcon_ai_sys", &[])
//            .with(NextActionApplierSystem, "char_control", &["friction_sys"]);
//        ecs_dispatcher_builder.add(
//            SavePreviousCharStateSystem,
//            "SavePreviousCharStateSystem",
//            &["char_control"],
//        );
//        ecs_dispatcher_builder = ecs_dispatcher_builder
//            .with(
//                CharacterStateUpdateSystem,
//                "char_state_update",
//                &["char_control"],
//            )
//            .with(
//                PhysCollisionCollectorSystem,
//                "collision_collector",
//                &["char_state_update"],
//            )
//            .with(SkillSystem, "skill_sys", &["collision_collector"])
//            .with(AttackSystem::new(), "attack_sys", &["collision_collector"]);
//        ecs_dispatcher_builder
//            .with_thread_local(ServerFrameEndSystem)
//            .build()
//    };
//    ecs_dispatcher
//}

pub fn console_print(
    ecs_world: &mut specs::World,
    text: &str,
    desktop_client_controller: ControllerEntityId,
) {
    log::debug!("{}", text);
    ecs_world
        .write_storage::<ConsoleComponent>()
        .get_mut(desktop_client_controller.into())
        .unwrap()
        .print(text);
}

fn register_systems<'a, 'b>(
    opengl_render_sys: Option<OpenGlRenderSystem<'b, 'b>>,
    maybe_sound_system: Option<SoundSystem>,
    console_system: Option<ConsoleSystem<'b>>,
    for_test: bool,
) -> Dispatcher<'a, 'b> {
    let ecs_dispatcher = {
        let mut ecs_dispatcher_builder = specs::DispatcherBuilder::new();
        let mut char_control_deps = vec!["friction_sys"];
        if !for_test {
            ecs_dispatcher_builder = ecs_dispatcher_builder
                .with(InputConsumerSystem, "input_handler", &[])
                .with(
                    InputToNextActionSystem::new(),
                    "input_to_next_action_sys",
                    &["input_handler"],
                )
                .with(
                    IntentionSenderSystem::new(),
                    "intention_sender",
                    &["input_to_next_action_sys"],
                )
                .with(CameraSystem, "camera_system", &["input_handler"]);
            char_control_deps.push("input_to_next_action_sys");
        }
        ecs_dispatcher_builder = ecs_dispatcher_builder.with(FrictionSystem, "friction_sys", &[]);
        ecs_dispatcher_builder = ecs_dispatcher_builder
            .with(MinionAiSystem, "minion_ai_sys", &[])
            .with(TurretAiSystem, "turret_ai_sys", &[])
            .with(FalconAiSystem, "falcon_ai_sys", &[])
            .with(
                NextActionApplierSystem,
                "char_control",
                char_control_deps.as_slice(),
            );
        if !for_test {
            ecs_dispatcher_builder.add(
                UpdateCharSpriteBasedOnStateSystem,
                "UpdateCharSpriteBasedOnStateSystem",
                &["char_control"],
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
                &["char_control"],
            );
        }
        ecs_dispatcher_builder = ecs_dispatcher_builder
            .with(
                CharacterStateUpdateSystem,
                "char_state_update",
                &["char_control"],
            )
            .with(
                PhysCollisionCollectorSystem,
                "collision_collector",
                &["char_state_update"],
            )
            .with(SkillSystem, "skill_sys", &["collision_collector"])
            .with(AttackSystem::new(), "attack_sys", &["collision_collector"])
            .with(SnapshotSystem::new(), "snapshot_sys", &["attack_sys"])
            .with(
                DebugServerAckComponentFillerSystem,
                "debug_ack",
                &["snapshot_sys"],
            );
        if let Some(console_system) = console_system {
            // thread_local to avoid Send fields
            ecs_dispatcher_builder = ecs_dispatcher_builder.with_thread_local(console_system);
        }
        if !for_test {
            ecs_dispatcher_builder = ecs_dispatcher_builder
                .with_thread_local(RenderDesktopClientSystem::new())
                .with_thread_local(FalconRenderSys)
                .with_thread_local(opengl_render_sys.unwrap());
        }
        if let Some(sound_system) = maybe_sound_system {
            ecs_dispatcher_builder = ecs_dispatcher_builder.with_thread_local(sound_system);
        }

        ecs_dispatcher_builder
            .with_thread_local(FrameCleanupSystem)
            .with_thread_local(FrameClientEndSystem)
            .build()
    };
    ecs_dispatcher
}

fn create_dispatcher_for_predictions<'a, 'b>() -> Dispatcher<'a, 'b> {
    return specs::DispatcherBuilder::new()
        .with_thread_local(NextActionApplierSystem)
        .with_thread_local(CharacterStateUpdateSystem)
        .with_thread_local(SnapshotSystem::new())
        .with_thread_local(FrameClientEndSystem)
        .build();
}

fn update_desktop_inputs(
    video: &mut Video,
    ecs_world: &mut World,
    desktop_client_controller: ControllerEntityId,
) -> bool {
    let mut storage = ecs_world.write_storage::<HumanInputComponent>();
    let inputs = storage.get_mut(desktop_client_controller.into()).unwrap();

    for event in video.event_pump.poll_iter() {
        //        video.imgui_sdl2.handle_event(&mut video.imgui, &event);
        match event {
            sdl2::event::Event::Quit { .. } => {
                return false;
            }
            _ => {
                inputs.inputs.push(event);
            }
        }
    }
    return true;
}

fn spawn_minions(ecs_world: &mut World) -> () {
    {
        let entity_id = create_random_char_minion(
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
        let entity_id = create_random_char_minion(
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

fn reload_configs_if_changed(
    runtime_conf_watcher_rx: crossbeam_channel::Receiver<Result<notify::Event, notify::Error>>,
    watcher: notify::RecommendedWatcher,
    ecs_world: &mut World,
) -> (
    crossbeam_channel::Receiver<Result<notify::Event, notify::Error>>,
    notify::RecommendedWatcher,
) {
    return match runtime_conf_watcher_rx.try_recv() {
        Ok(event) => {
            if let Ok(new_config) = DevConfig::new() {
                for input in (&mut ecs_world.write_storage::<HumanInputComponent>()).join() {
                    input.cast_mode = new_config.cast_mode
                }

                *ecs_world.write_resource::<DevConfig>() = new_config;
                for char_state in (&mut ecs_world.write_storage::<CharacterStateComponent>()).join()
                {
                    char_state.update_base_attributes(&ecs_world.write_resource::<DevConfig>());
                }

                log::info!("Configs has been reloaded");
            } else {
                log::warn!("Config error");
            }
            // On Linuxe, a "Remove" event is generated when a file is saved, which removes the active
            // watcher, so this code creates a new one
            if let Ok(notify::Event {
                kind: notify::EventKind::Remove(..),
                ..
            }) = event
            {
                let (tx, runtime_conf_watcher_rx) = crossbeam_channel::unbounded();
                let mut watcher = notify::watcher(tx, Duration::from_secs(2)).unwrap();
                watcher
                    .watch("config-runtime.toml", notify::RecursiveMode::NonRecursive)
                    .unwrap();
                (runtime_conf_watcher_rx, watcher)
            } else {
                (runtime_conf_watcher_rx, watcher)
            }
        }
        Err(crossbeam_channel::TryRecvError::Empty) => (runtime_conf_watcher_rx, watcher),
        Err(e) => {
            panic!("{:?}", e);
        }
    };
}

pub struct ConsoleCommandBuffer {
    commands: Vec<String>,
}

fn execute_console_commands(
    command_defs: &HashMap<String, CommandDefinition>,
    ecs_world: &mut World,
    desktop_client_controller: ControllerEntityId,
    video: &mut Video,
) {
    {
        let console_args = {
            let mut storage = ecs_world.write_storage::<ConsoleComponent>();
            let console = storage.get_mut(desktop_client_controller.into()).unwrap();
            std::mem::replace(&mut console.command_to_execute, None)
        };
        if let Some(cmd) = console_args {
            execute_console_command(
                cmd,
                command_defs,
                ecs_world,
                desktop_client_controller,
                video,
            );
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
            execute_console_command(
                cmd,
                command_defs,
                ecs_world,
                desktop_client_controller,
                video,
            );
        }
    }
    // run commands from config file, only 1 command per frame
    {
        let (line, skipped_line_count) = {
            let dev_config = ecs_world.write_resource::<DevConfig>();
            let without_useless_lines = dev_config
                .execute_script
                .lines()
                .skip_while(|line| line.starts_with("//") || line.trim().is_empty())
                .collect::<Vec<&str>>();
            let first_line: Option<String> = without_useless_lines.get(0).map(|it| it.to_string());
            let skipped_line_count =
                dev_config.execute_script.lines().count() - without_useless_lines.len();
            (first_line, skipped_line_count)
        };
        if let Some(command) = line {
            let cmd = CommandArguments::new(&command);
            execute_console_command(
                cmd,
                command_defs,
                ecs_world,
                desktop_client_controller,
                video,
            );
            let mut dev_config = ecs_world.write_resource::<DevConfig>();
            dev_config.execute_script = dev_config
                .execute_script
                .lines()
                .skip(skipped_line_count + 1)
                .collect::<Vec<&str>>()
                .join("\n");
        }
    }

    ecs_world.maintain();
}

fn execute_console_command(
    cmd: CommandArguments,
    command_defs: &HashMap<String, CommandDefinition>,
    ecs_world: &mut World,
    desktop_client_controller: ControllerEntityId,
    video: &mut Video,
) {
    let char_entity_id = {
        let storage = ecs_world.read_storage::<ControllerComponent>();
        let controller = storage.get(desktop_client_controller.into()).unwrap();
        controller.controlled_entity
    };
    log::debug!("Execute command: {:?}", cmd);
    let command_def = &command_defs[cmd.get_command_name().unwrap()];
    if let Err(e) = (command_def.action)(
        desktop_client_controller,
        char_entity_id,
        &cmd,
        ecs_world,
        video,
    ) {
        log::error!("Console error: {}", e);
        ecs_world
            .write_storage::<ConsoleComponent>()
            .get_mut(desktop_client_controller.into())
            .unwrap()
            .error(&e);
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

fn create_random_char_minion(ecs_world: &mut World, pos2d: Vec2, team: Team) -> CharEntityId {
    let mut rng = rand::thread_rng();
    let sex = if rng.gen::<usize>() % 2 == 0 {
        Sex::Male
    } else {
        Sex::Female
    };

    let (job_id, job_sprite_id) = if rng.gen::<usize>() % 2 == 0 {
        (JobId::SWORDMAN, JobSpriteId::SWORDMAN)
    } else {
        (JobId::ARCHER, JobSpriteId::ARCHER)
    };
    let head_count = ecs_world
        .read_resource::<SystemVariables>()
        .assets
        .sprites
        .head_sprites[Sex::Male as usize]
        .len();
    let char_entity_id = CharEntityId::from(ecs_world.create_entity().build());
    let updater = &ecs_world.read_resource::<LazyUpdate>();
    let head_index = rng.gen::<usize>() % head_count;
    CharacterEntityBuilder::new(char_entity_id, "minion")
        .insert_npc_component(updater)
        .insert_sprite_render_descr_component(updater)
        .physics(
            pos2d,
            &mut ecs_world.write_resource::<PhysicEngine>(),
            |builder| builder.collision_group(CollisionGroup::Minion).circle(1.0),
        )
        .char_state(
            updater,
            &ecs_world.read_resource::<DevConfig>(),
            pos2d,
            |ch| {
                ch.outlook(CharOutlook::Player {
                    sex,
                    job_sprite_id,
                    head_index,
                })
                .job_id(job_id)
                .team(team)
            },
        );
    char_entity_id
}
