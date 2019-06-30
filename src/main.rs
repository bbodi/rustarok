extern crate sdl2;
extern crate gl;
extern crate nalgebra;
extern crate encoding;
#[macro_use]
extern crate imgui;
extern crate imgui_sdl2;
extern crate imgui_opengl_renderer;
extern crate websocket;
#[macro_use]
extern crate log;
extern crate specs;
#[macro_use]
extern crate specs_derive;

use std::io::ErrorKind;
use crate::common::BinaryReader;
use crate::rsw::Rsw;
use crate::gnd::Gnd;
use crate::gat::Gat;

use imgui::ImString;
use nalgebra::{Vector3, Matrix4, Point3, Unit, Rotation3, Vector2, Point2};
use crate::video::{Shader, ShaderProgram, VertexArray, VertexAttribDefinition, GlTexture, Video};
use std::time::{Duration, SystemTime, Instant};
use std::collections::{HashMap, HashSet};
use crate::rsm::{Rsm, BoundingBox};
use sdl2::keyboard::{Keycode, Scancode};
use crate::act::ActionFile;
use crate::spr::SpriteFile;
use rand::Rng;
use websocket::stream::sync::TcpStream;
use websocket::{OwnedMessage, WebSocketError};
use log::LevelFilter;
use std::sync::Mutex;
use specs::Builder;
use specs::Join;
use specs::prelude::*;
use std::path::Path;
use crate::hardcoded_consts::job_name_table;
use crate::components::{CameraComponent, PositionComponent, InputProducerComponent, PhysicsComponent, BrowserClient, AnimatedSpriteComponent, DirectionComponent, DummyAiComponent};
use crate::systems::{SystemStopwatch, SystemVariables, SystemFrameDurations};
use crate::systems::render::{PhysicsDebugDrawingSystem, RenderBrowserClientsSystem, RenderStreamingSystem, RenderDesktopClientSystem};
use crate::systems::input::{InputConsumerSystem, BrowserInputProducerSystem};
use crate::systems::ai::DummyAiSystem;
use crate::systems::phys::PhysicsSystem;
use rand::prelude::ThreadRng;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::ColliderDesc;
use std::ops::Bound;
use ncollide2d::world::CollisionGroups;

// guild_vs4.rsw

//head sprite kirajzolása
//3xos gyorsitás = 1 frame alatt 3x annyi minden történik (3 physics etc

mod common;
mod cam;
mod video;
mod gat;
mod rsw;
mod gnd;
mod rsm;
mod act;
mod spr;
mod hardcoded_consts;

mod components;
mod systems;

enum ActionIndex {
    Idle = 0,
    Walking = 8,
    Sitting = 16,
    PickingItem = 24,
    StandBy = 32,
    Attacking1 = 40,
    ReceivingDamage = 48,
    Freeze1 = 56,
    Dead = 65,
    Freeze2 = 72,
    Attacking2 = 80,
    Attacking3 = 88,
    CastingSpell = 96,
}

const STATIC_MODELS_COLLISION_GROUP: usize = 1;
const LIVING_COLLISION_GROUP: usize = 2;

pub struct SpriteResource {
    action: ActionFile,
    frames: Vec<spr::RenderableFrame>,
}

impl SpriteResource {
    pub fn new(path: &str) -> SpriteResource {
        trace!("Loading {}", path);
        let frames: Vec<spr::RenderableFrame> = SpriteFile::load(
            BinaryReader::new(format!("{}.spr", path))
        ).frames
            .into_iter()
            .map(|frame| spr::RenderableFrame::from(frame))
            .collect();
        let action = ActionFile::load(
            BinaryReader::new(format!("{}.act", path))
        );
        SpriteResource {
            action,
            frames,
        }
    }
}


pub struct Shaders {
    pub ground_shader: ShaderProgram,
    pub model_shader: ShaderProgram,
    pub sprite_shader: ShaderProgram,
    pub trimesh_shader: ShaderProgram,
}

pub struct RenderMatrices {
    pub projection: Matrix4<f32>,
}

#[derive(Copy, Clone)]
pub struct Tick(u64);

#[derive(Copy, Clone)]
pub struct DeltaTime(f32);

fn main() {
    simple_logging::log_to_stderr(LevelFilter::Info);


    let mut video = Video::init();

    let shaders = Shaders {
        ground_shader: ShaderProgram::from_shaders(
            &[
                Shader::from_source(
                    include_str!("shaders/ground.vert"),
                    gl::VERTEX_SHADER,
                ).unwrap(),
                Shader::from_source(
                    include_str!("shaders/ground.frag"),
                    gl::FRAGMENT_SHADER,
                ).unwrap()
            ]
        ).unwrap(),
        model_shader: ShaderProgram::from_shaders(
            &[
                Shader::from_source(
                    include_str!("shaders/model.vert"),
                    gl::VERTEX_SHADER,
                ).unwrap(),
                Shader::from_source(
                    include_str!("shaders/model.frag"),
                    gl::FRAGMENT_SHADER,
                ).unwrap()
            ]
        ).unwrap(),
        sprite_shader: ShaderProgram::from_shaders(
            &[
                Shader::from_source(
                    include_str!("shaders/sprite.vert"),
                    gl::VERTEX_SHADER,
                ).unwrap(),
                Shader::from_source(
                    include_str!("shaders/sprite.frag"),
                    gl::FRAGMENT_SHADER,
                ).unwrap()
            ]
        ).unwrap(),
        trimesh_shader: ShaderProgram::from_shaders(
            &[
                Shader::from_source(
                    include_str!("shaders/trimesh.vert"),
                    gl::VERTEX_SHADER,
                ).unwrap(),
                Shader::from_source(
                    include_str!("shaders/trimesh.frag"),
                    gl::FRAGMENT_SHADER,
                ).unwrap()
            ]
        ).unwrap(),
    };

    let mut ecs_world = specs::World::new();
    ecs_world.register::<PositionComponent>();
    ecs_world.register::<CameraComponent>();
    ecs_world.register::<BrowserClient>();
    ecs_world.register::<InputProducerComponent>();
    ecs_world.register::<AnimatedSpriteComponent>();
    ecs_world.register::<DirectionComponent>();
    ecs_world.register::<DummyAiComponent>();
    ecs_world.register::<PhysicsComponent>();

    let desktop_client_entity = ecs_world
        .create_entity()
        .with(CameraComponent::new())
        .with(InputProducerComponent::default())
        .build();


    let mut ecs_dispatcher = specs::DispatcherBuilder::new()
        .with(BrowserInputProducerSystem, "browser_input_processor", &[])
        .with(InputConsumerSystem, "input_handler", &["browser_input_processor"])
        .with(DummyAiSystem, "ai", &[])
        .with(PhysicsSystem, "physics", &["ai", "input_handler", "browser_input_processor"])
        .with_thread_local(RenderBrowserClientsSystem)
        .with_thread_local(RenderStreamingSystem)
        .with_thread_local(RenderDesktopClientSystem)
        .with_thread_local(PhysicsDebugDrawingSystem::new())
        .build();

    let mut physics_world = nphysics2d::world::World::new();
    let map_render_data = load_map("prontera");
    for (model_name, matrix) in &map_render_data.model_instances {
        shaders.model_shader.set_mat4("model", &matrix);
        let model_render_data = &map_render_data.models[&model_name];
        let bbox = &model_render_data.bounding_box;

        let min = bbox.min;
        let min = matrix.transform_point(&Point3::new(min.x, 0.0, min.z));
        let max = bbox.max;
        let max = matrix.transform_point(&Point3::new(max.x, 0.0, max.z));
        let r: f32 = nalgebra::distance(&min, &max) / 2.0;

        let translation_vector = matrix.transform_point(&Point3::new(bbox.center.x, 0.0, bbox.center.z));

        let cuboid = ShapeHandle::new(
            ncollide2d::shape::Ball::new(r)
        );
        ColliderDesc::new(cuboid)
            .density(10.0)
            .translation(Vector2::new(translation_vector.x, translation_vector.z))
            .collision_groups(CollisionGroups::new()
                .with_membership(&[STATIC_MODELS_COLLISION_GROUP])
                .with_blacklist(&[STATIC_MODELS_COLLISION_GROUP])
                .with_whitelist(&[LIVING_COLLISION_GROUP]))
            .build(&mut physics_world);
    }

    fn grf(str: &str) -> String {
        format!("d:\\Games\\TalonRO\\grf\\data\\{}", str)
    }

    let mut rng = rand::thread_rng();

    let (elapsed, sprite_resources) = measure_time(|| {
        job_name_table().values().take(10).map(|job_name| {
            let male_file_name = grf("sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\") + job_name + "_³²";
            let female_file_name = grf("sprite\\ÀÎ°£Á·\\¸öÅë\\¿©\\") + job_name + "_¿©";
            let male = if Path::new(&(male_file_name.clone() + ".act")).exists() {
                Some(SpriteResource::new(&male_file_name))
            } else { None };
            let female = if Path::new(&(female_file_name.clone() + ".act")).exists() {
                Some(SpriteResource::new(&female_file_name))
            } else { None };
            vec![male, female]
        }).flatten().filter_map(|it| it).collect::<Vec<SpriteResource>>()
    });
    info!("act and spr files loaded[{}]: {}ms", sprite_resources.len(), elapsed.as_millis());

    let my_str = ImString::new("shitaka");

    let map_name_filter = ImString::new("prontera");
    let all_map_names = std::fs::read_dir("d:\\Games\\TalonRO\\grf\\data").unwrap().map(|entry| {
        let dir_entry = entry.unwrap();
        if dir_entry.file_name().into_string().unwrap().ends_with("rsw") {
            let mut sstr = dir_entry.file_name().into_string().unwrap();
            let len = sstr.len();
            sstr.truncate(len - 4); // remove extension
            Some(sstr)
        } else { None }
    }).filter_map(|x| x).collect::<Vec<String>>();

    let render_matrices = RenderMatrices {
        projection: Matrix4::new_perspective(std::f32::consts::FRAC_PI_4, 900f32 / 700f32, 0.1f32, 1000.0f32),
    };

    ecs_world.add_resource(SystemVariables {
        shaders,
        sprite_resources,
        tick: Tick(0),
        dt: DeltaTime(0.0),
        matrices: render_matrices,
        map_render_data,
        physics_world,
    });

    ecs_world.add_resource(SystemFrameDurations(HashMap::new()));

    let mut next_second: SystemTime = std::time::SystemTime::now().checked_add(Duration::from_secs(1)).unwrap();
    let mut last_tick_time: u64 = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
    let mut fps_counter: u64 = 0;
    let mut fps: u64 = 0;
    let mut time_multiplier = 1;


    let mut sent_bytes_per_second: usize = 0;
    let mut sent_bytes_per_second_counter: usize = 0;
    let mut websocket_server = websocket::sync::Server::bind("127.0.0.1:6969").unwrap();
    websocket_server.set_nonblocking(true).unwrap();

    let mut entity_count = 0;
    'running: loop {
        match websocket_server.accept() {
            Ok(wsupgrade) => {
                let browser_client = wsupgrade.accept().unwrap();
                browser_client.set_nonblocking(true).unwrap();
                info!("Client connected");
                ecs_world
                    .create_entity()
                    .with(CameraComponent::new())
                    .with(InputProducerComponent::default())
                    .with(BrowserClient {
                        websocket: Mutex::new(browser_client),
                        offscreen: vec![0; 900 * 700 * 4],
                        ping: 0,
                    })
                    .build();
            }
            _ => { /* Nobody tried to connect, move on.*/ }
        };

        {
            let mut storage = ecs_world.write_storage::<InputProducerComponent>();
            let inputs = storage.get_mut(desktop_client_entity).unwrap();

            for event in video.event_pump.poll_iter() {
                trace!("SDL event: {:?}", event);
                video.imgui_sdl2.handle_event(&mut video.imgui, &event);
                match event {
                    sdl2::event::Event::Quit { .. } | sdl2::event::Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running;
                    }
                    _ => {
                        inputs.inputs.push(event);
                    }
                }
            }
        }

        let tick = ecs_world.read_resource::<SystemVariables>().tick.0 as f32;

        ecs_dispatcher.dispatch(&mut ecs_world.res);
        ecs_world.maintain();

        imgui_frame(
            desktop_client_entity,
            &mut video,
            &mut ecs_world,
            rng.clone(),
            sent_bytes_per_second,
            &mut entity_count,
            &mut time_multiplier,
            fps,
        );

        video.gl_swap_window();

        let now = std::time::SystemTime::now();
        let now_ms = now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
        let dt = (now_ms - last_tick_time) as f32 / 1000.0 * time_multiplier as f32;
        last_tick_time = now_ms;
        if now >= next_second {
            fps = fps_counter;
            fps_counter = 0;
            sent_bytes_per_second = sent_bytes_per_second_counter;
            sent_bytes_per_second_counter = 0;
            next_second = std::time::SystemTime::now().checked_add(Duration::from_secs(1)).unwrap();

            video.set_title(&format!("Rustarok {} FPS", fps));

            // send a ping packet every second
            let now_ms = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
            let data = now_ms.to_le_bytes();
            let browser_storage = ecs_world.write_storage::<BrowserClient>();
            for browser_client in browser_storage.join() {
                let message = websocket::Message::ping(&data[..]);
                browser_client.websocket.lock().unwrap().send_message(&message).expect("Sending a ping message");
            }
        }
        fps_counter += 1;
        ecs_world.write_resource::<SystemVariables>().tick.0 += 1;
        ecs_world.write_resource::<SystemVariables>().dt.0 = dt;
    }
}

fn imgui_frame(desktop_client_entity: Entity,
               video: &mut Video,
               ecs_world: &mut World,
               mut rng: ThreadRng,
               sent_bytes_per_second: usize,
               entity_count: &mut i32,
               time_multiplier: &mut i32,
               fps: u64) {
    let ui = video.imgui_sdl2.frame(&video.window,
                                    &mut video.imgui,
                                    &video.event_pump.mouse_state());
//        extern crate sublime_fuzzy;
//        let map_name_filter_clone = map_name_filter.clone();
//        let filtered_map_names: Vec<&String> = all_map_names.iter()
//            .filter(|map_name| {
//                let matc = sublime_fuzzy::best_match(map_name_filter_clone.to_str(), map_name);
//                matc.is_some()
//            }).collect();
//        ui.window(im_str!("Maps: {},{},{}", camera.pos().x, camera.pos().y, camera.pos().z))
//            .position((0.0, 200.0), ImGuiCond::FirstUseEver)
//            .size((300.0, (100.0 + filtered_map_names.len() as f32 * 16.0).min(500.0)), ImGuiCond::Always)
//            .build(|| {
//                if ui.input_text(im_str!("Map name:"), &mut map_name_filter)
//                    .enter_returns_true(true)
//                    .build() {
//                    if let Some(map_name) = filtered_map_names.get(0) {
//                        map_render_data = load_map(map_name);
//                    }
//                }
//                for map_name in filtered_map_names.iter() {
//                    if ui.small_button(&ImString::new(map_name.as_str())) {
//                        map_render_data = load_map(map_name);
//                    }
//                }
//            });
    { // IMGUI
        ui.window(im_str!("Graphic opsions"))
            .position((0.0, 0.0), imgui::ImGuiCond::FirstUseEver)
            .size((300.0, 600.0), imgui::ImGuiCond::FirstUseEver)
            .build(|| {
                let mut map_render_data = &mut ecs_world.write_resource::<SystemVariables>().map_render_data;
                ui.checkbox(im_str!("Use tile_colors"), &mut map_render_data.use_tile_colors);
                if ui.checkbox(im_str!("Use use_lighting"), &mut map_render_data.use_lighting) {
                    map_render_data.use_lightmaps = map_render_data.use_lighting && map_render_data.use_lightmaps;
                }
                if ui.checkbox(im_str!("Use lightmaps"), &mut map_render_data.use_lightmaps) {
                    map_render_data.use_lighting = map_render_data.use_lighting || map_render_data.use_lightmaps;
                }
                ui.checkbox(im_str!("Models"), &mut map_render_data.draw_models);

                ui.slider_int(im_str!("Entities"), entity_count, 0, 300)
                    .build();
                ui.slider_int(im_str!("Speedup"), time_multiplier, 0, 3)
                    .build();

                ui.drag_float3(im_str!("light_dir"), &mut map_render_data.rsw.light.direction)
                    .min(-1.0).max(1.0).speed(0.05).build();
                ui.color_edit(im_str!("light_ambient"), &mut map_render_data.rsw.light.ambient)
                    .inputs(false)
                    .format(imgui::ColorFormat::Float)
                    .build();
                ui.color_edit(im_str!("light_diffuse"), &mut map_render_data.rsw.light.diffuse)
                    .inputs(false)
                    .format(imgui::ColorFormat::Float)
                    .build();
                ui.drag_float(im_str!("light_opacity"), &mut map_render_data.rsw.light.opacity)
                    .min(0.0).max(1.0).speed(0.05).build();

                let mut storage = ecs_world.write_storage::<CameraComponent>();
                let camera = storage.get(desktop_client_entity).unwrap();
                ui.text(im_str!("Maps: {},{},{}", camera.camera.pos().x, camera.camera.pos().y, camera.camera.pos().z));
                ui.text(im_str!("FPS: {}", fps));
                let (traffic, unit) = if sent_bytes_per_second > 1024 * 1024 {
                    (sent_bytes_per_second / 1024 / 1024, "Mb")
                } else if sent_bytes_per_second > 1024 {
                    (sent_bytes_per_second / 1024, "Kb")
                } else {
                    (sent_bytes_per_second, "bytes")
                };

                let system_frame_durations = &mut ecs_world.write_resource::<SystemFrameDurations>().0;
                ui.text(im_str!("Systems: "));
                for (sys_name, duration) in system_frame_durations.iter() {
                    let color = if *duration < 5 {
                        (0.0, 1.0, 0.0, 1.0)
                    } else if *duration < 10 {
                        (1.0, 0.8, 0.0, 1.0)
                    } else if *duration < 15 {
                        (1.0, 0.5, 0.0, 1.0)
                    } else if *duration < 20 {
                        (1.0, 0.2, 0.0, 1.0)
                    } else {
                        (1.0, 0.0, 0.0, 1.0)
                    };
                    ui.text_colored(color, im_str!("{}: {} ms", sys_name, duration));
                }
//                ui.text(im_str!("Traffic: {} {}", traffic, unit));
//
//                for browser_client in clients.iter() {
//                    ui.bullet_text(im_str!("Ping: {} ms", browser_client.ping));
//                }
            });
    }
    {
        let current_entity_count = ecs_world.read_storage::<AnimatedSpriteComponent>().join().count() as i32;
        if current_entity_count < *entity_count {
            for _i in 0..(*entity_count - current_entity_count) {
                let pos = {
                    let map_render_data = &ecs_world.read_resource::<SystemVariables>().map_render_data;
                    Point3::<f32>::new(2.0 * map_render_data.gnd.width as f32 * (rng.gen::<f32>()), 0.5, -(2.0 * map_render_data.gnd.height as f32 * (rng.gen::<f32>())))
                };
                let pos2d = Point2::new(pos.x, pos.z);
                let physics_component = {
                    let mut physics_world = &mut ecs_world.write_resource::<SystemVariables>().physics_world;
                    PhysicsComponent::new(&mut physics_world, pos2d.coords)
                };
                let sprite_count = ecs_world.read_resource::<SystemVariables>().sprite_resources.len();
                ecs_world
                    .create_entity()
                    .with(PositionComponent(pos.coords))
                    .with(DirectionComponent(0.0))
                    .with(physics_component)
                    .with(DummyAiComponent { target_pos: pos2d, state: 0 })
                    .with(AnimatedSpriteComponent {
                        file_index: rng.gen::<usize>() % sprite_count,
                        action_index: 8,
                        animation_start: Tick(0),
                        direction: 0,
                    })
                    .build();
            }
        } else if current_entity_count > *entity_count {
            let entities: Vec<_> = {
                let to_remove = current_entity_count - *entity_count;
                let entities_storage = ecs_world.entities();
                let sprite_storage = ecs_world.read_storage::<AnimatedSpriteComponent>(); // it is need only for filtering entities
                let physics_storage = ecs_world.read_storage::<PhysicsComponent>();
                (&entities_storage, &sprite_storage, &physics_storage).join()
                    .take(to_remove as usize)
                    .map(|(entity, _sprite_comp, phys_comp)| (entity, (*phys_comp).clone()))
                    .collect()
            };
            let entity_ids: Vec<_> = entities.iter().map(|(entity, _phys_comp)| *entity).collect();
            ecs_world.delete_entities(entity_ids.as_slice());

            // remove rigid bodies from the physic simulation
            let physics_world = &mut ecs_world.write_resource::<SystemVariables>().physics_world;
            let body_handles: Vec<_> = entities.iter().map(|(_entity, phys_comp)| phys_comp.handle).collect();
            physics_world.remove_bodies(body_handles.as_slice());
        }
    }
    video.renderer.render(ui);
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ModelName(String);

pub struct MapRenderData {
    pub gnd: Gnd,
    pub rsw: Rsw,
    pub light_wheight: [f32; 3],
    pub use_tile_colors: bool,
    pub use_lightmaps: bool,
    pub use_lighting: bool,
    pub ground_vertex_array: VertexArray,
    pub sprite_vertex_array: VertexArray,
    pub texture_atlas: GlTexture,
    pub tile_color_texture: GlTexture,
    pub lightmap_texture: GlTexture,
    pub models: HashMap<ModelName, ModelRenderData>,
    pub model_instances: Vec<(ModelName, Matrix4<f32>)>,
    pub draw_models: bool,
}

pub struct ModelRenderData {
    pub bounding_box: BoundingBox,
    pub alpha: f32,
    pub model: Vec<DataForRenderingSingleNode>,
}

pub struct EntityRenderData {
    pub pos: Vector3<f32>,
//    pub texture: GlTexture,
}

pub type DataForRenderingSingleNode = Vec<SameTextureNodeFaces>;

pub struct SameTextureNodeFaces {
    pub vao: VertexArray,
    pub texture: GlTexture,
}

pub fn measure_time<T, F: FnOnce() -> T>(f: F) -> (Duration, T) {
    let start = Instant::now();
    let r = f();
    (start.elapsed(), r)
}

fn load_map(map_name: &str) -> MapRenderData {
    let (elapsed, world) = measure_time(|| {
        Rsw::load(BinaryReader::new(format!("d:\\Games\\TalonRO\\grf\\data\\{}.rsw", map_name)))
    });
    info!("rsw loaded: {}ms", elapsed.as_millis());
    let (elapsed, _altitude) = measure_time(|| {
        Gat::load(BinaryReader::new(format!("d:\\Games\\TalonRO\\grf\\data\\{}.gat", map_name)))
    });
    info!("gat loaded: {}ms", elapsed.as_millis());
    let (elapsed, mut ground) = measure_time(|| {
        Gnd::load(BinaryReader::new(format!("d:\\Games\\TalonRO\\grf\\data\\{}.gnd", map_name)),
                  world.water.level,
                  world.water.wave_height)
    });
    info!("gnd loaded: {}ms", elapsed.as_millis());
    let (elapsed, models) = measure_time(|| {
        let model_names: HashSet<_> = world.models.iter().map(|m| m.filename.clone()).collect();
        Rsw::load_models(model_names)
    });
    info!("models[{}] loaded: {}ms", models.len(), elapsed.as_millis());

    let (elapsed, model_render_datas) = measure_time(|| {
        models.iter().map(|(name, rsm)| {
            let textures = Rsm::load_textures(&rsm.texture_names);
            let (data_for_rendering_full_model, bbox): (Vec<DataForRenderingSingleNode>, BoundingBox) = Rsm::generate_meshes_by_texture_id(
                &rsm.bounding_box,
                rsm.shade_type,
                rsm.nodes.len() == 1,
                &rsm.nodes,
                &textures,
            );
            (name.clone(), ModelRenderData {
                bounding_box: bbox,
                alpha: rsm.alpha,
                model: data_for_rendering_full_model,
            })
        }).collect::<HashMap<ModelName, ModelRenderData>>()
    });
    info!("model_render_datas loaded: {}ms", elapsed.as_millis());

    let model_instances: Vec<(ModelName, Matrix4<f32>)> = world.models.iter().map(|model_instance| {
        let mut instance_matrix = Matrix4::<f32>::identity();
        instance_matrix.prepend_translation_mut(&(model_instance.pos + Vector3::new(ground.width as f32, 0f32, ground.height as f32)));

// rot_z
        let rotation = Rotation3::from_axis_angle(&Unit::new_normalize(Vector3::z()), model_instance.rot.z.to_radians()).to_homogeneous();
        instance_matrix = instance_matrix * rotation;
// rot x
        let rotation = Rotation3::from_axis_angle(&Unit::new_normalize(Vector3::x()), model_instance.rot.x.to_radians()).to_homogeneous();
        instance_matrix = instance_matrix * rotation;
// rot y
        let rotation = Rotation3::from_axis_angle(&Unit::new_normalize(Vector3::y()), model_instance.rot.y.to_radians()).to_homogeneous();
        instance_matrix = instance_matrix * rotation;

        instance_matrix.prepend_nonuniform_scaling_mut(&model_instance.scale);

        let rotation = Rotation3::from_axis_angle(&Unit::new_normalize(Vector3::x()), 180f32.to_radians()).to_homogeneous();
        instance_matrix = rotation * instance_matrix;

        (model_instance.filename.clone(), instance_matrix)
    }).collect();

    let (elapsed, texture_atlas) = measure_time(|| {
        Gnd::create_gl_texture_atlas(&ground.texture_names)
    });
    info!("model texture_atlas loaded: {}ms", elapsed.as_millis());

    let tile_color_texture = Gnd::create_tile_color_texture(
        &mut ground.tiles_color_image,
        ground.width, ground.height,
    );
    let lightmap_texture = Gnd::create_lightmap_texture(&ground.lightmap_image, ground.lightmaps.count);

    let s: Vec<[f32; 4]> = vec![
        [-0.5, 0.5, 0.0, 0.0],
        [0.5, 0.5, 1.0, 0.0],
        [-0.5, -0.5, 0.0, 1.0],
        [0.5, -0.5, 1.0, 1.0]
    ];
    let sprite_vertex_array = VertexArray::new(
        gl::TRIANGLE_STRIP,
        &s, 4, None, vec![
            VertexAttribDefinition {
                number_of_components: 2,
                offset_of_first_element: 0,
            }, VertexAttribDefinition { // uv
                number_of_components: 2,
                offset_of_first_element: 2,
            }
        ]);

    let ground_vertex_array = VertexArray::new(
        gl::TRIANGLES,
        &ground.mesh, ground.mesh.len(), None, vec![
            VertexAttribDefinition {
                number_of_components: 3,
                offset_of_first_element: 0,
            }, VertexAttribDefinition { // normals
                number_of_components: 3,
                offset_of_first_element: 3,
            }, VertexAttribDefinition { // texcoords
                number_of_components: 2,
                offset_of_first_element: 6,
            }, VertexAttribDefinition { // lightmap_coord
                number_of_components: 2,
                offset_of_first_element: 8,
            }, VertexAttribDefinition { // tile color coordinate
                number_of_components: 2,
                offset_of_first_element: 10,
            }
        ]);
    MapRenderData {
        gnd: ground,
        rsw: world,
        ground_vertex_array,
        models: model_render_datas,
        texture_atlas,
        tile_color_texture,
        lightmap_texture,
        model_instances,
        sprite_vertex_array,
        use_tile_colors: true,
        use_lightmaps: true,
        use_lighting: true,
        draw_models: true,
        light_wheight: [0f32; 3],
    }
}