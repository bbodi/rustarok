extern crate actix_web;
extern crate byteorder;
extern crate config;
extern crate crossbeam_channel;
extern crate encoding;
#[macro_use]
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2;
extern crate libflate;
extern crate log;
extern crate nalgebra;
extern crate notify;
extern crate sdl2;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate specs;
#[macro_use]
extern crate specs_derive;
extern crate strum;
extern crate strum_macros;
extern crate sublime_fuzzy;
extern crate vek;
extern crate websocket;

use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use imgui::{ImString, ImVec2};
use log::LevelFilter;
use nalgebra::{Matrix4, Point2};
use rand::Rng;
use specs::prelude::*;
use specs::Builder;
use specs::Join;

use crate::asset::database::AssetDatabase;
use crate::asset::{AssetLoader, SpriteResource};
use crate::common::{measure_time, DeltaTime, ElapsedTime};
use crate::components::char::{
    CharActionIndex, CharOutlook, CharType, CharacterStateComponent, NpcComponent,
    SpriteRenderDescriptorComponent, Team,
};
use crate::components::controller::{
    CameraComponent, CharEntityId, ControllerComponent, ControllerEntityId, HumanInputComponent,
};
use crate::components::skills::skills::SkillManifestationComponent;
use crate::components::{BrowserClient, MinionComponent};
use crate::configs::{AppConfig, DevConfig};
use crate::consts::{JobId, JobSpriteId};
use crate::network::{handle_client_handshakes, handle_new_connections};
use crate::notify::Watcher;
use crate::runtime_assets::audio::init_audio_and_load_sounds;
use crate::runtime_assets::ecs::create_ecs_world;
use crate::runtime_assets::effect::load_str_effects;
use crate::runtime_assets::graphic::{
    load_skill_icons, load_sprites, load_status_icons, load_texts,
};
use crate::runtime_assets::map::{load_map, CollisionGroup, PhysicEngine};
use crate::shaders::load_shaders;
use crate::systems::atk_calc::AttackSystem;
use crate::systems::camera_system::CameraSystem;
use crate::systems::char_state_sys::CharacterStateUpdateSystem;
use crate::systems::console_system::{
    CommandArguments, CommandDefinition, ConsoleComponent, ConsoleSystem,
};
use crate::systems::falcon_ai_sys::{
    FalconAiSystem, FalconComponent, FalconState, FALCON_FLY_HEIGHT,
};
use crate::systems::frame_end_system::FrameEndSystem;
use crate::systems::input_sys::{BrowserInputProducerSystem, InputConsumerSystem};
use crate::systems::input_to_next_action::InputToNextActionSystem;
use crate::systems::minion_ai_sys::MinionAiSystem;
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::phys::{FrictionSystem, PhysCollisionCollectorSystem};
use crate::systems::render::falcon_render_sys::FalconRenderSys;
use crate::systems::render::opengl_render_sys::OpenGlRenderSystem;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render::websocket_browser_render_sys::WebSocketBrowserRenderSystem;
use crate::systems::render_sys::RenderDesktopClientSystem;
use crate::systems::skill_sys::SkillSystem;
use crate::systems::turret_ai_sys::TurretAiSystem;
use crate::systems::{
    CollisionsFromPrevFrame, RenderMatrices, Sex, SystemFrameDurations, SystemVariables,
};
use crate::video::{Video, VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::web_server::start_web_server;

#[macro_use]
mod common;
mod asset;
mod audio;
mod cam;
mod configs;
mod consts;
mod cursor;
mod effect;
mod my_gl;
mod network;
mod runtime_assets;
mod shaders;
mod video;
mod web_server;

#[macro_use]
mod components;
mod systems;

// TODO: throttle: if browser is not able to keep up e.g. 140 FPS, slow down render command sending

// simulations per second
pub const SIMULATION_FREQ: u64 = 30;
pub const MAX_SECONDS_ALLOWED_FOR_SINGLE_FRAME: f32 = (1000 / SIMULATION_FREQ) as f32 / 1000.0;

//  csak a camera felé néző falak rajzolódjanak ilyenkor ki
//  a modelleket z sorrendben növekvőleg rajzold ki
//jobIDt tartalmazzon ne indexet a sprite
// guild_vs4.rsw
//3xos gyorsitás = 1 frame alatt 3x annyi minden történik (3 physics etc

fn main() {
    let config = AppConfig::new().expect("Could not load config file ('config.toml')");
    let (tx, runtime_conf_watcher_rx) = crossbeam_channel::unbounded();
    let mut watcher = notify::watcher(tx, Duration::from_secs(2)).unwrap();
    watcher
        .watch("config-runtime.toml", notify::RecursiveMode::NonRecursive)
        .unwrap();

    simple_logging::log_to_stderr(
        LevelFilter::from_str(&config.log_level)
            .expect("Unknown log level. Please set one of the following values for 'log_level' in 'config.toml': \"OFF\", \"ERROR\", \"WARN\", \"INFO\", \"DEBUG\", \"TRACE\"")
    );
    let (elapsed, asset_loader) = measure_time(|| {
        AssetLoader::new(config.grf_paths.as_slice())
            .expect("Could not open asset files. Please configure them in 'config.toml'")
    });
    log::info!("GRF loading: {}ms", elapsed.as_millis());

    let mut asset_database = AssetDatabase::new();

    let mut map_name_filter = ImString::new("prontera");
    let mut filtered_map_names: Vec<String> = vec![];
    let all_map_names = get_all_map_names(&asset_loader);
    let all_str_names = get_all_effect_names(&asset_loader);

    let mut fov = 0.638;
    let mut window_opened = false;
    let mut cam_angle = -60.0;
    let render_matrices = RenderMatrices::new(fov);

    let sdl_context = sdl2::init().unwrap();
    // !!! gl_context: sdl2::video::GLContext THIS MUST BE KEPT IN SCOPE, DON'T REMOVE IT!
    let (mut video, gl, _gl_context) = Video::init(&sdl_context);

    let map_name = "prontera";
    let (map_render_data, physics_world) = load_map(
        &gl,
        map_name,
        &asset_loader,
        &mut asset_database,
        config.quick_startup,
    );

    let command_defs: HashMap<String, CommandDefinition> =
        ConsoleSystem::init_commands(all_str_names);

    let mut ecs_world = create_ecs_world();

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();
    let (str_effects, str_effect_cache) = load_str_effects(&gl, &asset_loader, &mut asset_database);
    let opengl_render_sys = OpenGlRenderSystem::new(gl.clone(), &ttf_context, str_effect_cache);
    let (maybe_sound_system, sounds) = init_audio_and_load_sounds(&sdl_context, &asset_loader);
    let system_vars = SystemVariables::new(
        load_sprites(&gl, &asset_loader, &mut asset_database),
        load_texts(&gl, &ttf_context, &mut asset_database),
        load_shaders(&gl),
        render_matrices,
        map_render_data,
        load_status_icons(&gl, &asset_loader, &mut asset_database),
        load_skill_icons(&gl, &asset_loader, &mut asset_database),
        str_effects,
        sounds,
        asset_loader,
        gl,
    );

    let mut ecs_dispatcher = {
        let mut ecs_dispatcher_builder = specs::DispatcherBuilder::new()
            .with(BrowserInputProducerSystem, "browser_input_processor", &[])
            .with(
                InputConsumerSystem,
                "input_handler",
                &["browser_input_processor"],
            )
            .with(CameraSystem, "camera_system", &["input_handler"])
            .with(FrictionSystem, "friction_sys", &[])
            .with(
                InputToNextActionSystem,
                "input_to_next_action_sys",
                &["input_handler", "browser_input_processor"],
            )
            .with(MinionAiSystem, "minion_ai_sys", &[])
            .with(TurretAiSystem, "turret_ai_sys", &[])
            .with(FalconAiSystem, "falcon_ai_sys", &[])
            //////////////////////////////////////
            // statuses
            /////////////////////////////////////
            //            .with(
            //                CharStatusCleanerSysem,
            //                "CharStatusCleanerSysem",
            //                &["input_handler"],
            //            )
            /////////////////////////////////////
            //            .with(StunStatusSystem, "StunStatusSystem", &["input_handler"])
            /////////////////////////////////////
            // statuses end
            /////////////////////////////////////
            .with(
                NextActionApplierSystem,
                "char_control",
                &[
                    "friction_sys",
                    "input_to_next_action_sys",
                    "browser_input_processor",
                ],
            )
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
            .with(AttackSystem, "attack_sys", &["collision_collector"])
            .with_thread_local(ConsoleSystem::new(&command_defs)) // thread_local to avoid Send fields
            .with_thread_local(RenderDesktopClientSystem::new())
            .with_thread_local(FalconRenderSys)
            .with_thread_local(opengl_render_sys)
            .with_thread_local(WebSocketBrowserRenderSystem::new());
        if let Some(sound_system) = maybe_sound_system {
            ecs_dispatcher_builder = ecs_dispatcher_builder.with_thread_local(sound_system);
        }

        ecs_dispatcher_builder
            .with_thread_local(FrameEndSystem)
            .build()
    };
    ecs_world.add_resource(system_vars);
    ecs_world.add_resource(DevConfig::new().unwrap());
    ecs_world.add_resource(RenderCommandCollector::new());

    ecs_world.add_resource(asset_database);

    ecs_world.add_resource(CollisionsFromPrevFrame {
        collisions: HashMap::new(),
    });

    ecs_world.add_resource(physics_world);
    ecs_world.add_resource(SystemFrameDurations(HashMap::new()));
    let desktop_client_char = CharEntityId(ecs_world.create_entity().build());
    let desktop_client_controller = ControllerEntityId(ecs_world.create_entity().build());
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
        Point2::new(
            ecs_world.read_resource::<DevConfig>().start_pos_x,
            ecs_world.read_resource::<DevConfig>().start_pos_y,
        ),
        Sex::Male,
        JobId::CRUSADER,
        1,
        1,
        Team::Right,
        &ecs_world.read_resource::<DevConfig>(),
    );
    ecs_world
        .read_resource::<LazyUpdate>()
        .insert(desktop_client_controller.0, ConsoleComponent::new());

    // add falcon to it
    let start_x = ecs_world.read_resource::<DevConfig>().start_pos_x;
    let start_y = ecs_world.read_resource::<DevConfig>().start_pos_y;
    let _falcon_id = ecs_world
        .create_entity()
        .with(FalconComponent::new(desktop_client_char, start_x, start_y))
        .with(SpriteRenderDescriptorComponent {
            action_index: CharActionIndex::Idle as usize,
            fps_multiplier: 1.0,
            animation_started: ElapsedTime(0.0),
            forced_duration: None,
            direction: 0,
            animation_ends_at: ElapsedTime(0.0),
        })
        .build();

    ecs_world.maintain();

    let mut next_second: SystemTime = std::time::SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap();
    let mut last_tick_time: u64 = get_current_ms(SystemTime::now());
    let mut next_minion_spawn = ElapsedTime(2.0);
    let mut fps_counter: u64 = 0;
    let mut fps: u64 = 0;
    let mut fps_history: Vec<f32> = Vec::with_capacity(30);
    let mut system_frame_durations = SystemFrameDurations(HashMap::new());

    let mut websocket_server = websocket::sync::Server::bind("0.0.0.0:6969").unwrap();
    websocket_server.set_nonblocking(true).unwrap();

    // Add static skill manifestations
    {
        for command in &[
            "spawn_area poison 50 2 3 2000 500 251 -213",
            "spawn_area absorb 50 2 3 2000 3000 255 -213",
            "spawn_area firebomb 50 2 3 2000 2000 260 -213",
            "spawn_area armor 70 2 3 2000 10000 265 -213",
            "spawn_area armor -30 2 3 2000 10000 270 -213",
            "spawn_area heal 50 2 3 500 0 273 -213",
            "spawn_entity dummy_enemy left 1 217 -66",
            "spawn_entity dummy_enemy left 1 219 -66",
            "spawn_entity dummy_enemy left 1 221 -66",
            "spawn_entity dummy_ally left 1 219 -71",
            // RIGHT TEAM GUARDS
            // middle final 4 guards on lamps
            "spawn_entity guard right 1 246 -214 GEFFEN_MAGE_12 3.5",
            "spawn_entity guard right 1 266 -214 GEFFEN_MAGE_12 3.5",
            "spawn_entity guard right 1 246 -194 GEFFEN_MAGE_12 3.5",
            "spawn_entity guard right 1 266 -194 GEFFEN_MAGE_12 3.5",
            // middle, middle 3 guards on bridge
            "spawn_entity guard right 1 195 -208 GEFFEN_MAGE_12 6",
            "spawn_entity guard right 1 195 -204 GEFFEN_MAGE_12 6",
            "spawn_entity guard right 1 195 -200 GEFFEN_MAGE_12 6",
            // top, guard alone on lamp
            "spawn_entity guard right 1 200 -326 GEFFEN_MAGE_12 3.5",
            // top, two guards on lamps
            "spawn_entity guard right 1 186 -299 GEFFEN_MAGE_12 3.5",
            "spawn_entity guard right 1 200 -299 GEFFEN_MAGE_12 3.5",
            // LEFT TEAM GUARDS
            // middle final 4 guards on lamps
            "spawn_entity guard left 1 66 -214 GEFFEN_MAGE_9 3.5",
            "spawn_entity guard left 1 48 -214 GEFFEN_MAGE_9 3.5",
            "spawn_entity guard left 1 66 -194 GEFFEN_MAGE_9 3.5",
            "spawn_entity guard left 1 48 -194 GEFFEN_MAGE_9 3.5",
            // middle, middle 3 guards on bridge
            "spawn_entity guard left 1 117 -208 GEFFEN_MAGE_9 6",
            "spawn_entity guard left 1 117 -204 GEFFEN_MAGE_9 6",
            "spawn_entity guard left 1 117 -200 GEFFEN_MAGE_9 6",
            // top, guard alone on lamp
            "spawn_entity guard left 1 112 -326 GEFFEN_MAGE_9 3.5",
            // top, two guards on lamps
            "spawn_entity guard left 1 112 -299 GEFFEN_MAGE_9 3.5",
            "spawn_entity guard left 1 126 -299 GEFFEN_MAGE_9 3.5",
        ] {
            execute_console_command(
                command,
                &command_defs,
                &mut ecs_world,
                desktop_client_char,
                desktop_client_controller,
            );
            ecs_world.maintain();
        }
    }

    start_web_server();

    'running: loop {
        handle_new_connections(map_name, &mut ecs_world, &mut websocket_server);

        handle_client_handshakes(&mut ecs_world);

        let quit = !update_desktop_inputs(&mut video, &mut ecs_world, desktop_client_controller);
        if quit {
            break 'running;
        }

        ecs_dispatcher.dispatch(&mut ecs_world.res);
        run_console_commands(
            &command_defs,
            &mut ecs_world,
            desktop_client_char,
            desktop_client_controller,
        );
        execute_finished_skill_castings(&mut ecs_world);
        ecs_world.maintain();

        let (new_map, show_cursor) = imgui_frame(
            desktop_client_controller,
            &mut video,
            &mut ecs_world,
            &mut map_name_filter,
            &all_map_names,
            &mut filtered_map_names,
            fps,
            fps_history.as_slice(),
            &mut fov,
            &mut cam_angle,
            &mut window_opened,
            &system_frame_durations,
        );
        sdl_context.mouse().show_cursor(show_cursor);
        if let Some(new_map_name) = new_map {
            ecs_world.delete_all();
            let (map_render_data, physics_world) = load_map(
                &ecs_world.read_resource::<SystemVariables>().gl,
                &new_map_name,
                &ecs_world.read_resource::<SystemVariables>().asset_loader,
                &mut ecs_world.write_resource::<AssetDatabase>(),
                config.quick_startup,
            );
            ecs_world
                .write_resource::<SystemVariables>()
                .map_render_data = map_render_data;
            ecs_world.add_resource(physics_world);

            // TODO
        }

        video.gl_swap_window();

        std::thread::sleep(Duration::from_millis(
            ecs_world.read_resource::<DevConfig>().sleep_ms,
        ));
        let now = std::time::SystemTime::now();
        let now_ms = get_current_ms(now);
        let dt = (now_ms - last_tick_time) as f32 / 1000.0;
        last_tick_time = now_ms;
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

            send_ping_packets(&mut ecs_world)
        }
        fps_counter += 1;
        ecs_world
            .write_resource::<SystemVariables>()
            .update_timers(dt);

        let now = ecs_world.read_resource::<SystemVariables>().time;
        if next_minion_spawn.has_already_passed(now)
            && ecs_world.read_resource::<DevConfig>().minions_enabled
        {
            next_minion_spawn = now.add_seconds(2.0);
            spawn_minions(&mut ecs_world)
        }

        // runtime configs
        reload_configs_if_changed(&runtime_conf_watcher_rx, &mut ecs_world);
    }
}

fn update_desktop_inputs(
    video: &mut Video,
    ecs_world: &mut specs::world::World,
    desktop_client_controller: ControllerEntityId,
) -> bool {
    let mut storage = ecs_world.write_storage::<HumanInputComponent>();
    let inputs = storage.get_mut(desktop_client_controller.0).unwrap();

    for event in video.event_pump.poll_iter() {
        video.imgui_sdl2.handle_event(&mut video.imgui, &event);
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

pub fn get_current_ms(now: SystemTime) -> u64 {
    now.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn send_ping_packets(ecs_world: &mut specs::world::World) -> () {
    let now_ms = get_current_ms(SystemTime::now());
    let data = now_ms.to_le_bytes();
    let mut browser_storage = ecs_world.write_storage::<BrowserClient>();
    let camera_storage = ecs_world.read_storage::<CameraComponent>();

    for (browser_client, _no_camera) in (&mut browser_storage, &camera_storage).join() {
        browser_client.send_ping(&data);
        browser_client.reset_byte_per_second();
    }
}

fn spawn_minions(ecs_world: &mut specs::world::World) -> () {
    {
        let char_entity_id = create_random_char_minion(
            ecs_world,
            p2!(
                MinionAiSystem::CHECKPOINTS[0][0],
                MinionAiSystem::CHECKPOINTS[0][1]
            ),
            Team::Right,
        );
        ecs_world
            .create_entity()
            .with(ControllerComponent::new(char_entity_id))
            .with(MinionComponent { fountain_up: false });
    }
    {
        let entity_id = create_random_char_minion(
            ecs_world,
            p2!(
                MinionAiSystem::CHECKPOINTS[5][0],
                MinionAiSystem::CHECKPOINTS[5][1]
            ),
            Team::Left,
        );
        let mut storage = ecs_world.write_storage();
        storage
            .insert(entity_id.0, MinionComponent { fountain_up: false })
            .unwrap();
    }
}

fn reload_configs_if_changed(
    runtime_conf_watcher_rx: &crossbeam_channel::Receiver<Result<notify::Event, notify::Error>>,
    ecs_world: &mut specs::world::World,
) {
    match runtime_conf_watcher_rx.try_recv() {
        Ok(_event) => {
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
        }
        _ => {}
    };
}

fn execute_console_command(
    command: &str,
    command_defs: &HashMap<String, CommandDefinition>,
    ecs_world: &mut specs::world::World,
    desktop_client_char: CharEntityId,
    desktop_client_controller: ControllerEntityId,
) {
    {
        let args = CommandArguments::new(command);
        let command_def = &command_defs[args.get_command_name().unwrap()];
        let _result = (command_def.action)(
            desktop_client_controller,
            desktop_client_char,
            &args,
            ecs_world,
        );
    }
}

fn execute_finished_skill_castings(ecs_world: &mut specs::world::World) {
    // TODO: avoid allocating new vec
    let finished_casts = std::mem::replace(
        &mut ecs_world
            .write_resource::<SystemVariables>()
            .just_finished_skill_casts,
        Vec::with_capacity(128),
    );
    for finished_cast in &finished_casts {
        let manifestation = finished_cast.skill.get_definition().finish_cast(
            finished_cast.caster_entity_id,
            finished_cast.caster_pos,
            finished_cast.skill_pos,
            &finished_cast.char_to_skill_dir,
            finished_cast.target_entity,
            ecs_world,
        );
        if let Some(manifestation) = manifestation {
            let skill_entity_id = ecs_world.create_entity().build();
            ecs_world.read_resource::<LazyUpdate>().insert(
                skill_entity_id,
                SkillManifestationComponent::new(skill_entity_id, manifestation),
            );
        }
    }
}

fn run_console_commands(
    command_defs: &HashMap<String, CommandDefinition>,
    ecs_world: &mut specs::world::World,
    desktop_client_char: CharEntityId,
    desktop_client_controller: ControllerEntityId,
) {
    let console_args = {
        let mut storage = ecs_world.write_storage::<ConsoleComponent>();
        let console = storage.get_mut(desktop_client_controller.0).unwrap();
        std::mem::replace(&mut console.command_to_execute, None)
    };
    if let Some(args) = console_args {
        let command_def = &command_defs[args.get_command_name().unwrap()];
        if let Err(e) = (command_def.action)(
            desktop_client_controller,
            desktop_client_char,
            &args,
            ecs_world,
        ) {
            ecs_world
                .write_storage::<ConsoleComponent>()
                .get_mut(desktop_client_controller.0)
                .unwrap()
                .error(&e);
        }
    }
}

fn get_all_effect_names(asset_loader: &AssetLoader) -> Vec<String> {
    let all_str_names = asset_loader
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

fn get_all_map_names(asset_loader: &AssetLoader) -> Vec<String> {
    let all_map_names = asset_loader
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

fn imgui_frame(
    desktop_client_entity: ControllerEntityId,
    video: &mut Video,
    ecs_world: &mut specs::world::World,
    mut map_name_filter: &mut ImString,
    all_map_names: &Vec<String>,
    filtered_map_names: &mut Vec<String>,
    fps: u64,
    fps_history: &[f32],
    fov: &mut f32,
    cam_angle: &mut f32,
    window_opened: &mut bool,
    system_frame_durations: &SystemFrameDurations,
) -> (Option<String>, bool) {
    let ui = video.imgui_sdl2.frame(
        &video.window,
        &mut video.imgui,
        &video.event_pump.mouse_state(),
    );
    let mut ret = (None, false); // (map, show_cursor)
    {
        // IMGUI
        ui.window(im_str!("Graphic options"))
            .position((0.0, 0.0), imgui::ImGuiCond::FirstUseEver)
            .size((300.0, 600.0), imgui::ImGuiCond::FirstUseEver)
            .opened(window_opened)
            .build(|| {
                ret.1 = ui.is_window_hovered();
                let map_name_filter_clone = map_name_filter.clone();
                if ui
                    .input_text(im_str!("Map name:"), &mut map_name_filter)
                    .enter_returns_true(false)
                    .build()
                {
                    filtered_map_names.clear();
                    filtered_map_names.extend(
                        all_map_names
                            .iter()
                            .filter(|map_name| {
                                let matc = sublime_fuzzy::best_match(
                                    map_name_filter_clone.to_str(),
                                    map_name,
                                );
                                matc.is_some()
                            })
                            .map(|it| it.to_owned()),
                    );
                }
                for map_name in filtered_map_names.iter() {
                    if ui.small_button(&ImString::new(map_name.as_str())) {
                        ret.0 = Some(map_name.to_owned());
                    }
                }

                if ui
                    .slider_float(im_str!("Perspective"), fov, 0.1, std::f32::consts::PI)
                    .build()
                {
                    ecs_world
                        .write_resource::<SystemVariables>()
                        .matrices
                        .projection = Matrix4::new_perspective(
                        VIDEO_WIDTH as f32 / VIDEO_HEIGHT as f32,
                        *fov,
                        0.1f32,
                        1000.0f32,
                    );
                }

                if ui
                    .slider_float(im_str!("Camera"), cam_angle, -120.0, 120.0)
                    .build()
                {
                    let mut storage = ecs_world.write_storage::<CameraComponent>();
                    let controller = storage.get_mut(desktop_client_entity.0).unwrap();
                    controller.camera.rotate(*cam_angle, 270.0);
                }

                let mut map_render_data = &mut ecs_world
                    .write_resource::<SystemVariables>()
                    .map_render_data;
                ui.checkbox(
                    im_str!("Use tile_colors"),
                    &mut map_render_data.use_tile_colors,
                );
                if ui.checkbox(
                    im_str!("Use use_lighting"),
                    &mut map_render_data.use_lighting,
                ) {
                    map_render_data.use_lightmaps =
                        map_render_data.use_lighting && map_render_data.use_lightmaps;
                }
                if ui.checkbox(im_str!("Use lightmaps"), &mut map_render_data.use_lightmaps) {
                    map_render_data.use_lighting =
                        map_render_data.use_lighting || map_render_data.use_lightmaps;
                }
                ui.checkbox(im_str!("Models"), &mut map_render_data.draw_models);
                ui.checkbox(im_str!("Ground"), &mut map_render_data.draw_ground);

                let camera = ecs_world
                    .read_storage::<CameraComponent>()
                    .get(desktop_client_entity.0)
                    .unwrap()
                    .clone();
                let mut storage = ecs_world.write_storage::<HumanInputComponent>();

                {
                    let controller = storage.get_mut(desktop_client_entity.0).unwrap();
                    ui.text(im_str!(
                        "Mouse world pos: {}, {}",
                        controller.mouse_world_pos.x,
                        controller.mouse_world_pos.y,
                    ));
                }

                ui.drag_float3(
                    im_str!("light_dir"),
                    &mut map_render_data.rsw.light.direction,
                )
                .min(-1.0)
                .max(1.0)
                .speed(0.05)
                .build();
                ui.color_edit(
                    im_str!("light_ambient"),
                    &mut map_render_data.rsw.light.ambient,
                )
                .inputs(false)
                .format(imgui::ColorFormat::Float)
                .build();
                ui.color_edit(
                    im_str!("light_diffuse"),
                    &mut map_render_data.rsw.light.diffuse,
                )
                .inputs(false)
                .format(imgui::ColorFormat::Float)
                .build();
                ui.drag_float(
                    im_str!("light_opacity"),
                    &mut map_render_data.rsw.light.opacity,
                )
                .min(0.0)
                .max(1.0)
                .speed(0.05)
                .build();

                ui.text(im_str!(
                    "Maps: {},{},{}",
                    camera.camera.pos().x,
                    camera.camera.pos().y,
                    camera.camera.pos().z
                ));
                ui.text(im_str!("yaw: {}, pitch: {}", camera.yaw, camera.pitch));
                ui.text(im_str!("FPS: {}", fps));

                ui.plot_histogram(im_str!("FPS"), fps_history)
                    .scale_min(100.0)
                    .scale_max(145.0)
                    .graph_size(ImVec2::new(0.0f32, 200.0f32))
                    .build();
                ui.text(im_str!("Systems[micro sec]: "));
                for (sys_name, durations) in system_frame_durations.0.iter() {
                    let diff = (durations.max / 100) as f32 / (durations.min / 100).max(1) as f32;

                    let color = if diff < 1.5 && durations.avg < 5000 {
                        (0.0, 1.0, 0.0, 1.0)
                    } else if diff < 2.0 && durations.avg < 5000 {
                        (1.0, 0.75, 0.0, 1.0)
                    } else if diff < 2.5 && durations.avg < 5000 {
                        (1.0, 0.5, 0.0, 1.0)
                    } else if durations.avg < 5000 {
                        (1.0, 0.25, 0.0, 1.0)
                    } else {
                        (1.0, 0.0, 0.0, 1.0)
                    };
                    ui.text_colored(
                        color,
                        im_str!(
                            "{}: {}, {}, {}",
                            sys_name,
                            durations.min,
                            durations.max,
                            durations.avg
                        ),
                    );
                }
                let browser_storage = ecs_world.read_storage::<BrowserClient>();
                for browser_client in browser_storage.join() {
                    ui.bullet_text(im_str!("Ping: {} ms", browser_client.ping));
                }
            });
    }
    video.renderer.render(ui);
    return ret;
}

fn create_random_char_minion(
    ecs_world: &mut specs::world::World,
    pos2d: Point2<f32>,
    team: Team,
) -> CharEntityId {
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
    let entity_id = CharEntityId(ecs_world.create_entity().build());
    ecs_world
        .read_resource::<LazyUpdate>()
        .insert(entity_id.0, NpcComponent);
    components::char::attach_char_components(
        "minion".to_owned(),
        entity_id,
        &ecs_world.read_resource::<LazyUpdate>(),
        &mut ecs_world.write_resource::<PhysicEngine>(),
        pos2d,
        0.0,
        CharOutlook::Player {
            sex,
            job_sprite_id,
            head_index: rng.gen::<usize>() % head_count,
        },
        job_id,
        1,
        team,
        CharType::Minion,
        CollisionGroup::NonPlayer,
        &[
            //CollisionGroup::NonPlayer,
            CollisionGroup::Player,
            CollisionGroup::StaticModel,
            CollisionGroup::NonCollidablePlayer,
        ],
        &ecs_world.read_resource::<DevConfig>(),
    );
    entity_id
}
