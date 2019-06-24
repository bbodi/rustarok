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
use nalgebra::{Vector3, Matrix4, Point3, Unit, Rotation3};
use crate::opengl::{Shader, ShaderProgram, VertexArray, VertexAttribDefinition, GlTexture};
use std::time::{Duration, SystemTime, Instant};
use std::collections::{HashMap, HashSet};
use crate::rsm::Rsm;
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
use ncollide3d::shape::ShapeHandle;
use nphysics3d::object::{ColliderDesc, RigidBodyDesc};
use ncollide3d::procedural::TriMesh;

// guild_vs4.rsw

mod common;
mod opengl;
mod gat;
mod rsw;
mod gnd;
mod rsm;
mod act;
mod spr;
mod hardcoded_consts;

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

pub struct Camera {
    pub pos: Point3<f32>,
    pub front: Vector3<f32>,
    pub up: Vector3<f32>,
    pub right: Vector3<f32>,
}

// the values that should be added to the sprite direction based on the camera
// direction (the index is the camera direction, which is floor(angle/45)
const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];

impl Camera {
    pub fn new(pos: Point3<f32>) -> Camera {
        let front = Vector3::<f32>::new(0.0, 0.0, -1.0);
        let up = Vector3::<f32>::y();
        Camera {
            pos,
            front,
            up,
            right: front.cross(&up).normalize(),
        }
    }

    pub fn pos(&self) -> Point3<f32> {
        self.pos
    }

    pub fn rotate(&mut self, pitch: f32, yaw: f32) {
        self.front = Vector3::<f32>::new(
            pitch.to_radians().cos() * yaw.to_radians().cos(),
            pitch.to_radians().sin(),
            pitch.to_radians().cos() * yaw.to_radians().sin(),
        ).normalize();
        self.right = self.front.cross(&Vector3::y()).normalize();
        self.up = self.right.cross(&self.front).normalize();
    }

    pub fn move_forward(&mut self, speed: f32) {
        self.pos += speed * self.front;
    }

    pub fn move_side(&mut self, speed: f32) {
        self.pos += self.front.cross(&self.up).normalize() * speed;
    }

    pub fn create_view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.pos, &(self.pos + self.front), &self.up)
    }

    pub fn look_at(&mut self, p: Point3<f32>) {
        self.front = (p - self.pos).normalize();
        self.right = self.front.cross(&Vector3::y()).normalize();
        self.up = self.right.cross(&self.front).normalize();
    }
}

#[derive(Component)]
struct CameraComponent {
    camera: Camera,
    mouse_down: bool,
    last_mouse_x: u16,
    last_mouse_y: u16,
    yaw: f32,
    pitch: f32,
}


impl CameraComponent {
    fn new() -> CameraComponent {
        CameraComponent {
            camera: Camera::new(Point3::new(0.0, 0.0, 3.0)),
            mouse_down: false,
            last_mouse_x: 400,
            last_mouse_y: 300,
            yaw: 270.0,
            pitch: 0.0,
        }
    }
}

#[derive(Component)]
struct BrowserClient {
    websocket: Mutex<websocket::sync::Client<TcpStream>>,
    offscreen: Vec<u8>,
    ping: u16,
}

#[derive(Component)]
pub struct PositionComponent(Vector3<f32>);

#[derive(Component, Default)]
pub struct InputProducerComponent {
    inputs: Vec<sdl2::event::Event>,
    keys: HashSet<Scancode>,
}

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

struct PhysicsSystem;

impl<'a> specs::System<'a> for PhysicsSystem {
    type SystemData = (
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        mut system_vars,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("PhysicsSystem");
        system_vars.physics_world.step();
    }
}


#[derive(Component)]
struct DummyAiComponent {
    target_pos: Point3<f32>,
    state: i32, // 0 standing, 1 walking
}

struct DummyAiSystem;

struct SystemStopwatch<'a> {
    // let now_ms = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    started: Instant,
    name: &'static str,
    times: &'a mut SystemFrameDurations,
}

impl<'a> SystemStopwatch<'a> {
    pub fn new(name: &'static str, times: &'a mut SystemFrameDurations) -> SystemStopwatch<'a> {
        SystemStopwatch {
            started: Instant::now(),
            name,
            times,
        }
    }
}

impl<'a> Drop for SystemStopwatch<'a> {
    fn drop(&mut self) {
        self.times.system_finished(self.started, self.name);
    }
}


impl<'a> specs::System<'a> for DummyAiSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, PositionComponent>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, DummyAiComponent>,
        specs::WriteStorage<'a, AnimatedSpriteComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        entities,
        mut position_storage,
        mut physics_storage,
        mut ai_storage,
        mut animated_sprite_storage,
        mut system_vars,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("DummyAiSystem");
        let mut rng = rand::thread_rng();
        for (entity, pos, ai, physics_comp) in (&entities, &mut position_storage, &mut ai_storage, &mut physics_storage).join() {
            let mut body = system_vars.physics_world.rigid_body_mut(physics_comp.handle).unwrap();
            let pos = body.position().translation.vector;
            if nalgebra::distance(&nalgebra::Point::from(pos), &ai.target_pos) < 10.0 {
                ai.target_pos = Point3::<f32>::new(0.5 * 200.0 * (rng.gen::<f32>()), 0.5, -(0.5 * 200.0 * (rng.gen::<f32>())));
                if let Some(anim_sprite) = animated_sprite_storage.get_mut(entity) {
                    let dir_vec = ai.target_pos - pos;
                    // "- 90.0"
                    // The calculated yaw for the camera are 90 at [0;1] and 180 at [1;0] etc,
                    // this calculation gives a different result which is shifted 90 degrees clockwise,
                    // so it is 90 at [1;0].
                    let dd = dir_vec.x.atan2(dir_vec.z).to_degrees() - 90.0;
                    let dd = if dd < 0.0 { dd + 360.0 } else if dd > 360.0 { dd - 360.0 } else { dd };
                    let dir_index = (dd / 45.0 + 0.5) as usize % 8;
                    anim_sprite.direction = DIRECTION_TABLE[dir_index];
                }
            } else {
                let mut force = (ai.target_pos - nalgebra::Point::from(pos)).normalize() * 10.0;
                body.set_linear_velocity(force);
            }
        }
    }
}


struct BrowserInputProducerSystem;

impl<'a> specs::System<'a> for BrowserInputProducerSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, InputProducerComponent>,
        specs::WriteStorage<'a, BrowserClient>,
    );

    fn run(&mut self, (
        entities,
        mut input_storage,
        mut browser_client_storage,
    ): Self::SystemData) {
        for (entity, client, input_producer) in (&entities, &mut browser_client_storage, &mut input_storage).join() {
            let sh = client.websocket.lock().unwrap().recv_message();
            if let Ok(msg) = sh {
                match msg {
                    OwnedMessage::Pong(buf) => {
                        let ping_time = u128::from_le_bytes([
                            buf[0], buf[1], buf[2], buf[3],
                            buf[4], buf[5], buf[6], buf[7],
                            buf[8], buf[9], buf[10], buf[11],
                            buf[12], buf[13], buf[14], buf[15],
                        ]);
                        let now_ms = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
                        client.ping = (now_ms - ping_time) as u16;
                    }
                    OwnedMessage::Binary(buf) => {
                        let mut iter = buf.iter();
                        while let Some(header) = iter.next() {
                            match header {
                                1 => {
                                    let upper_byte = iter.next().unwrap();
                                    let lower_byte = iter.next().unwrap();
                                    let mouse_x: u16 = ((*upper_byte as u16) << 8) | *lower_byte as u16;

                                    let upper_byte = iter.next().unwrap();
                                    let lower_byte = iter.next().unwrap();
                                    let mouse_y: u16 = ((*upper_byte as u16) << 8) | *lower_byte as u16;
                                    trace!("Message arrived: MouseMove({}, {})", mouse_x, mouse_y);
                                    let shit2 = (0 as u32,
                                                 0 as i32,
                                                 0 as i32);
                                    let shit = unsafe { std::mem::transmute(shit2) };
                                    input_producer.inputs.push(
                                        sdl2::event::Event::MouseMotion {
                                            timestamp: 0,
                                            window_id: 0,
                                            which: 0,
                                            mousestate: shit,
                                            x: mouse_x as i32,
                                            y: mouse_y as i32,
                                            xrel: 0,
                                            yrel: 0,
                                        }
                                    );
                                }
                                2 => {
                                    trace!("Message arrived: MouseDown");
                                    input_producer.inputs.push(
                                        sdl2::event::Event::MouseButtonDown {
                                            timestamp: 0,
                                            window_id: 0,
                                            which: 0,
                                            mouse_btn: sdl2::mouse::MouseButton::Left,
                                            clicks: 0,
                                            x: 0,
                                            y: 0,
                                        }
                                    );
                                }
                                3 => {
                                    trace!("Message arrived: MouseUp");
                                    input_producer.inputs.push(
                                        sdl2::event::Event::MouseButtonUp {
                                            timestamp: 0,
                                            window_id: 0,
                                            which: 0,
                                            mouse_btn: sdl2::mouse::MouseButton::Left,
                                            clicks: 0,
                                            x: 0,
                                            y: 0,
                                        });
                                }
                                4 => {
                                    let scancode = *iter.next().unwrap();
                                    let upper_byte = *iter.next().unwrap();
                                    let lower_byte = *iter.next().unwrap();
                                    let input_char: u16 = ((upper_byte as u16) << 8) | lower_byte as u16;
                                    trace!("Message arrived: KeyDown({}, {})", scancode, input_char);
                                    input_producer.inputs.push(
                                        sdl2::event::Event::KeyDown {
                                            timestamp: 0,
                                            window_id: 0,
                                            keycode: None,
                                            scancode: Scancode::from_i32(scancode as i32),
                                            keymod: sdl2::keyboard::Mod::NOMOD,
                                            repeat: false,
                                        });
                                    if let Some(ch) = std::char::from_u32(input_char as u32) {
                                        input_producer.inputs.push(
                                            sdl2::event::Event::TextInput {
                                                timestamp: 0,
                                                window_id: 0,
                                                text: ch.to_string(),
                                            }
                                        );
                                    }
                                }
                                5 => {
                                    let scancode = *iter.next().unwrap();
                                    trace!("Message arrived: KeyUp({})", scancode);
                                    input_producer.inputs.push(
                                        sdl2::event::Event::KeyUp {
                                            timestamp: 0,
                                            window_id: 0,
                                            keycode: None,
                                            scancode: Scancode::from_i32(scancode as i32),
                                            keymod: sdl2::keyboard::Mod::NOMOD,
                                            repeat: false,
                                        });
                                }
                                _ => {
                                    warn!("Unknown header: {}", header);
                                    entities.delete(entity).unwrap();
                                }
                            };
                        }
                    }
                    _ => {
                        warn!("Unknown msg: {:?}", msg);
                        entities.delete(entity).unwrap();
                    }
                }
            } else if let Err(WebSocketError::IoError(e)) = sh {
                if e.kind() == ErrorKind::ConnectionAborted {
                    // 10053, ConnectionAborted
                    info!("Client has disconnected");
                    entities.delete(entity).unwrap();
                }
            }
        }
    }
}

struct InputConsumerSystem;

impl<'a> specs::System<'a> for InputConsumerSystem {
    type SystemData = (
        specs::WriteStorage<'a, InputProducerComponent>,
        specs::WriteStorage<'a, CameraComponent>,
    );

    fn run(&mut self, (
        mut input_storage,
        mut camera_storage,
    ): Self::SystemData) {
        for (client, input_producer) in (&mut camera_storage, &mut input_storage).join() {
            let events: Vec<_> = input_producer.inputs.drain(..).collect();
            for event in events {
                match event {
                    sdl2::event::Event::MouseButtonDown { .. } => {
                        client.mouse_down = true;
                    }
                    sdl2::event::Event::MouseButtonUp { .. } => {
                        client.mouse_down = false;
                    }
                    sdl2::event::Event::MouseMotion {
                        timestamp: _,
                        window_id: _,
                        which: _,
                        mousestate: _,
                        x,
                        y,
                        xrel: _,
                        yrel: _
                    } => {
                        if client.mouse_down {
                            let x_offset = x - client.last_mouse_x as i32;
                            let y_offset = client.last_mouse_y as i32 - y; // reversed since y-coordinates go from bottom to top
                            client.yaw += x_offset as f32;
                            client.pitch += y_offset as f32;
                            if client.pitch > 89.0 {
                                client.pitch = 89.0;
                            }
                            if client.pitch < -89.0 {
                                client.pitch = -89.0;
                            }
                            if client.yaw > 360.0 {
                                client.yaw -= 360.0;
                            } else if client.yaw < 0.0 {
                                client.yaw += 360.0;
                            }
                            client.camera.rotate(client.pitch, client.yaw);
                        }
                        client.last_mouse_x = x as u16;
                        client.last_mouse_y = y as u16;
                    }
                    sdl2::event::Event::KeyDown { scancode, .. } => {
                        if scancode.is_some() {
                            input_producer.keys.insert(scancode.unwrap());
                        }
                    }
                    sdl2::event::Event::KeyUp { scancode, .. } => {
                        if scancode.is_some() {
                            input_producer.keys.remove(&scancode.unwrap());
                        }
                    }
                    _ => {}
                }

                let camera_speed = if input_producer.keys.contains(&Scancode::LShift) { 6.0 } else { 2.0 };
                if input_producer.keys.contains(&Scancode::W) {
                    client.camera.move_forward(camera_speed);
                } else if input_producer.keys.contains(&Scancode::S) {
                    client.camera.move_forward(-camera_speed);
                }
                if input_producer.keys.contains(&Scancode::A) {
                    client.camera.move_side(-camera_speed);
                } else if input_producer.keys.contains(&Scancode::D) {
                    client.camera.move_side(camera_speed);
                }
            }
        }
    }
}

struct RenderBrowserClientsSystem;

#[derive(Component)]
struct DirectionComponent(f32);

#[derive(Component)]
struct AnimatedSpriteComponent {
    file_index: usize,
    action_index: usize,
    animation_start: Tick,
    direction: usize,
}

impl<'a> specs::System<'a> for RenderBrowserClientsSystem {
    type SystemData = (
        specs::ReadStorage<'a, CameraComponent>,
        specs::WriteStorage<'a, BrowserClient>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(&mut self, (
        camera_storage,
        mut browser_client_storage,
        system_vars,
    ): Self::SystemData) {
        for (camera, browser) in (&camera_storage, &mut browser_client_storage).join() {
            let view = camera.camera.create_view_matrix();
//            render_client(
//                &view,
//                &system_vars.shaders.ground_shader_program,
//                &system_vars.shaders.model_shader_program,
//                &system_vars.shaders.sprite_shader_program,
//                &system_vars.matrices.projection,
//                &system_vars.map_render_data,
//            );
            // now the back buffer contains the rendered image for this client
            unsafe {
                gl::ReadBuffer(gl::BACK);
                gl::ReadPixels(0, 0, 900, 700, gl::RGBA, gl::UNSIGNED_BYTE, browser.offscreen.as_mut_ptr() as *mut gl::types::GLvoid);
            }
        }
    }
}

struct RenderDesktopClientSystem;

struct SystemVariables {
    shaders: Shaders,
    sprite_resources: Vec<SpriteResource>,
    tick: Tick,
    matrices: RenderMatrices,
    map_render_data: MapRenderData,
    physics_world: nphysics3d::world::World<f32>,
}

struct SystemFrameDurations(HashMap<&'static str, u32>);

impl SystemFrameDurations {
    pub fn system_finished(&mut self, started: Instant, name: &'static str) {
        let duration = Instant::now().duration_since(started).as_millis() as u32;
        self.0.insert(name, duration);
    }

    pub fn start_measurement(&mut self, name: &'static str) -> SystemStopwatch {
        SystemStopwatch::new(name, self)
    }
}

impl<'a> specs::System<'a> for RenderDesktopClientSystem {
    type SystemData = (
        specs::ReadStorage<'a, CameraComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PositionComponent>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, DirectionComponent>,
        specs::ReadStorage<'a, AnimatedSpriteComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        camera_storage,
        browser_client_storage,
        position_storage,
        physics_storage,
        dir_storage,
        animated_sprite_storage,
        system_vars,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("RenderDesktopClientSystem");
        for (camera, _not_browser) in (&camera_storage, !&browser_client_storage).join() {
            let view = camera.camera.create_view_matrix();
            render_client(
                &view,
                &system_vars.shaders,
                &system_vars.matrices.projection,
                &system_vars.map_render_data,
            );
            system_vars.shaders.sprite_shader.gl_use();
            system_vars.shaders.sprite_shader.set_mat4("projection", &system_vars.matrices.projection);
            system_vars.shaders.sprite_shader.set_mat4("view", &view);
            system_vars.shaders.sprite_shader.set_int("model_texture", 0);
            system_vars.shaders.sprite_shader.set_f32("alpha", 1.0);

            let binded_sprite_vertex_array = system_vars.map_render_data.sprite_vertex_array.bind();


            for (_entity_pos, physics, dir, animated_sprite) in (&position_storage,
                                                                 &physics_storage,
                                                                 &dir_storage,
                                                                 &animated_sprite_storage).join() {
                // draw layer
                let tick = system_vars.tick;
                let animation_elapsed_tick = tick.0 - animated_sprite.animation_start.0;
                let cam_dir = (((camera.yaw / 45.0) + 0.5) as usize) % 8;
                let idx = animated_sprite.action_index + (animated_sprite.direction + DIRECTION_TABLE[cam_dir]) % 8;
                let resource = &system_vars.sprite_resources[animated_sprite.file_index];
                let delay = resource.action.actions[idx].delay;
                let frame_count = resource.action.actions[idx].frames.len();
                let frame_index = ((animation_elapsed_tick / (delay / 20) as u64) % frame_count as u64) as usize;
                for layer in &resource.action.actions[idx].frames[frame_index].layers {
                    if layer.sprite_frame_index < 0 {
                        continue;
                    }
                    let sprite_frame = &resource.frames[layer.sprite_frame_index as usize];

                    let width = sprite_frame.original_width as f32 * layer.scale[0];
                    let height = sprite_frame.original_height as f32 * layer.scale[1];
                    sprite_frame.texture.bind(gl::TEXTURE0);

                    let mut matrix = Matrix4::<f32>::identity();
//                    let mut pos = entity_pos.0;
                    let body = system_vars.physics_world.rigid_body(physics.handle).unwrap();
                    let mut pos = body.position().translation.vector;
                    pos.x += layer.pos[0] as f32 / 175.0 * 5.0;
                    pos.y -= layer.pos[1] as f32 / 175.0 * 5.0;
                    matrix.prepend_translation_mut(&pos);

                    system_vars.shaders.sprite_shader.set_mat4("model", &matrix);
                    let width = width as f32 / 175.0 * 5.0;
                    let width = if layer.is_mirror { -width } else { width };
                    system_vars.shaders.sprite_shader.set_vec3("size", &[
                        width,
                        height as f32 / 175.0 * 5.0,
                        0.0
                    ]);
                    system_vars.shaders.sprite_shader.set_f32("alpha", 1.0);

                    binded_sprite_vertex_array.draw(gl::TRIANGLE_STRIP);
                }
            }
        }
    }
}

struct PhysicsDebugDrawingSystem {
    capsule_vertex_array: VertexArray,
}

impl PhysicsDebugDrawingSystem {
    fn new() -> PhysicsDebugDrawingSystem {
        let mut capsule_mesh = ncollide3d::procedural::capsule(
            &2.0f32,
            &4.0f32,
            100, 100);

        capsule_mesh.unify_index_buffer();
//        let capsule_mesh = ncollide3d::shape::TriMesh::from(capsule_mesh);
        let indices: Vec<u32> = capsule_mesh.indices.unwrap_unified().into_iter().map(|idx| {
            idx[0]
        }).collect();
        let capsule_vertex_array = VertexArray::new(
            &capsule_mesh.coords,
            indices.len(),
            Some(indices.as_slice()),
            vec![
                VertexAttribDefinition {
                    number_of_components: 3,
                    offset_of_first_element: 0,
                }
            ]);
        PhysicsDebugDrawingSystem {
            capsule_vertex_array,
        }
    }
}

impl<'a> specs::System<'a> for PhysicsDebugDrawingSystem {
    type SystemData = (
        specs::ReadStorage<'a, CameraComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        camera_storage,
        browser_client_storage,
        physics_storage,
        system_vars,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("PhysicsDebugDrawingSystem");
        for (camera, _not_browser) in (&camera_storage, !&browser_client_storage).join() {
            let view = camera.camera.create_view_matrix();

            for physics in (&physics_storage).join() {
                let mut matrix = Matrix4::<f32>::identity();
                let body = system_vars.physics_world.rigid_body(physics.handle).unwrap();
                let pos = body.position().translation.vector;
                matrix.prepend_translation_mut(&pos);

                system_vars.shaders.trimesh_shader.gl_use();
                system_vars.shaders.trimesh_shader.set_mat4("projection", &system_vars.matrices.projection);
                system_vars.shaders.trimesh_shader.set_mat4("view", &view);
                system_vars.shaders.trimesh_shader.set_f32("alpha", 1.0);
                system_vars.shaders.trimesh_shader.set_mat4("model", &matrix);
                self.capsule_vertex_array.bind().draw(gl::TRIANGLES);
            }
        }
    }
}


struct RenderStreamingSystem;

impl<'a> specs::System<'a> for RenderStreamingSystem {
    type SystemData = (
        specs::WriteStorage<'a, BrowserClient>,
    );

    fn run(&mut self, (
        browser_client_storage,
    ): Self::SystemData) {
        for browser in (&browser_client_storage).join() {
            let message = websocket::Message::binary(browser.offscreen.as_slice());
//                sent_bytes_per_second_counter += client.offscreen.len();
            // it is ok if it fails, the client might have disconnected but
            // ecs_world.maintain has not executed yet to remove it from the world
            let _result = browser.websocket.lock().unwrap().send_message(&message);
        }
    }
}

struct Shaders {
    ground_shader: ShaderProgram,
    model_shader: ShaderProgram,
    sprite_shader: ShaderProgram,
    trimesh_shader: ShaderProgram,
}

struct RenderMatrices {
    projection: Matrix4<f32>,
}

#[derive(Copy, Clone)]
struct Tick(u64);

#[derive(Component, Clone)]
struct PhysicsComponent {
    handle: nphysics3d::object::BodyHandle
}

impl PhysicsComponent {
    fn new(world: &mut nphysics3d::world::World<f32>,
           pos: Vector3<f32>) -> PhysicsComponent {
        let capsule = ShapeHandle::new(ncollide3d::shape::Capsule::new(2.0, 1.0));
        let mut collider_desc = ColliderDesc::new(capsule)
            .density(1.3);
        let mut rb_desc = RigidBodyDesc::new().collider(&collider_desc);
        let handle = rb_desc
            .set_translation(pos)
            .build(world).handle();
        PhysicsComponent {
            handle
        }
    }
}


fn main() {
    simple_logging::log_to_stderr(LevelFilter::Debug);

    let mut physics_world = nphysics3d::world::World::new();


    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let gl_attr = video_subsystem.gl_attr();

    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 5);
    let mut window = video_subsystem
        .window("Rustarok", 900, 700)
        .opengl()
        .allow_highdpi()
        .resizable()
        .build()
        .unwrap();

    // these two variables must be in scope, so don't remove their variables
    let _gl_context = window.gl_create_context().unwrap();
    let _gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    unsafe {
        gl::Viewport(0, 0, 900, 700); // set viewport
        gl::ClearColor(0.3, 0.3, 0.5, 1.0);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

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

    let mut map_render_data = load_map("prontera");
    let mut rng = rand::thread_rng();


    fn grf(str: &str) -> String {
        format!("d:\\Games\\TalonRO\\grf\\data\\{}", str)
    }
    // data\.act
//    let sprite_resources = vec![
//        SpriteResource::new(&grf("sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\°Ç³Ê_³²")), // Male Gunslinger
//        SpriteResource::new(&grf("sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\±¸ÆäÄÚÅ©·ç¼¼ÀÌ´õ_³²")), // Male Peco Crusader
//        SpriteResource::new(&grf("sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\±Â»Ç_H_³²")), // Male Knight
//        SpriteResource::new(&grf("sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\¹«ÈÑ¹ÙÁÖ_³²")), // Female bard
//        SpriteResource::new(&grf("sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\¾Î¼¼½Å_³²")), // Male assassin
//        SpriteResource::new(&grf("sprite\\ÀÎ°£Á·\\¸öÅë\\¿©\\Å©·Ç¼¼ÀÌ´Õ_H_¿©")), // Female crusader
//    ];

    // return 'data/sprite/\xc0\xce\xb0\xa3\xc1\xb7 / \xb8\xf6\xc5\xeb/' +
    // SexTable[sex] + '/' +
    // (ClassTable[id] || ClassTable[0]) + '_'
    // + SexTable[sex];

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

//    let sprite_resources = std::fs::read_dir(grf("sprite\\ÀÎ°£Á·\\¸öÅë\\³²")).unwrap().map(|entry| {
//        let dir_entry = entry.unwrap();
//        if dir_entry.file_name().into_string().unwrap().ends_with("act") {
//            let mut sstr = dir_entry.file_name().into_string().unwrap();
//            let len = sstr.len();
//            sstr.truncate(len - 4); // remove extension
//            Some(sstr)
//        } else { None }
//    }).filter_map(|x| x.map(|it| SpriteResource::new(&(grf("sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\") + &it))))
//        .collect::<Vec<SpriteResource>>();


    let mut imgui = imgui::ImGui::init();
    imgui.set_ini_filename(None);
    let video = sdl_context.video().unwrap();
    let mut imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui);

    let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, |s| video.gl_get_proc_address(s) as _);

    let mut event_pump = sdl_context.event_pump().unwrap();

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
        matrices: render_matrices,
        map_render_data,
        physics_world,
    });

    ecs_world.add_resource(SystemFrameDurations(HashMap::new()));

    let mut next_second: SystemTime = std::time::SystemTime::now().checked_add(Duration::from_secs(1)).unwrap();
    let mut fps_counter: u64 = 0;
    let mut fps: u64 = 0;


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
            _ => {
// Nobody tried to connect, move on.
            }
        };

        {
            let mut storage = ecs_world.write_storage::<InputProducerComponent>();
            let inputs = storage.get_mut(desktop_client_entity).unwrap();
            for event in event_pump.poll_iter() {
                trace!("SDL event: {:?}", event);
                imgui_sdl2.handle_event(&mut imgui, &event);
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


// Imgui logic
        let ui = imgui_sdl2.frame(&window, &mut imgui, &event_pump.mouse_state());

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

                    ui.slider_int(im_str!("Entities"), &mut entity_count, 0, 300)
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

                    ui.text(im_str!("FPS: {}", fps));
                    let (traffic, unit) = if sent_bytes_per_second > 1024 * 1024 {
                        (sent_bytes_per_second / 1024 / 1024, "Mb")
                    } else if sent_bytes_per_second > 1024 {
                        (sent_bytes_per_second / 1024, "Kb")
                    } else {
                        (sent_bytes_per_second, "bytes")
                    };

                    let mut system_frame_durations = &mut ecs_world.write_resource::<SystemFrameDurations>().0;
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
            if current_entity_count < entity_count {
                for _i in 0..(entity_count - current_entity_count) {
                    let pos = {
                        let map_render_data = &ecs_world.read_resource::<SystemVariables>().map_render_data;
                        Point3::<f32>::new(2.0 * map_render_data.gnd.width as f32 * (rng.gen::<f32>()), 0.5, -(2.0 * map_render_data.gnd.height as f32 * (rng.gen::<f32>())))
                    };
                    let physics_component = {
                        let mut physics_world = &mut ecs_world.write_resource::<SystemVariables>().physics_world;
                        PhysicsComponent::new(&mut physics_world, pos.coords)
                    };
                    let sprite_count = ecs_world.read_resource::<SystemVariables>().sprite_resources.len();
                    ecs_world
                        .create_entity()
                        .with(PositionComponent(pos.coords))
                        .with(DirectionComponent(0.0))
                        .with(physics_component)
                        .with(DummyAiComponent { target_pos: pos, state: 0 })
                        .with(AnimatedSpriteComponent {
                            file_index: rng.gen::<usize>() % sprite_count,
                            action_index: 8,
                            animation_start: Tick(0),
                            direction: 0,
                        })
                        .build();
                }
            } else if current_entity_count > entity_count {
                let entities: Vec<_> = {
                    let to_remove = current_entity_count - entity_count;
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

                // remove rigid body from the physic simulation
                let mut physics_world = &mut ecs_world.write_resource::<SystemVariables>().physics_world;
                let body_handles: Vec<_> = entities.iter().map(|(_entity, phys_comp)| phys_comp.handle).collect();
                physics_world.remove_bodies(body_handles.as_slice());
            }
        }

        let tick = ecs_world.read_resource::<SystemVariables>().tick.0 as f32;

        ecs_dispatcher.dispatch(&mut ecs_world.res);
        ecs_world.maintain();

        renderer.render(ui);

        window.gl_swap_window();
        if std::time::SystemTime::now() >= next_second {
            fps = fps_counter;
            fps_counter = 0;
            sent_bytes_per_second = sent_bytes_per_second_counter;
            sent_bytes_per_second_counter = 0;
            next_second = std::time::SystemTime::now().checked_add(Duration::from_secs(1)).unwrap();
            window.set_title(&format!("Rustarok {} FPS", fps)).unwrap();

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
    }
}

fn render_client(view: &Matrix4<f32>,
                 shaders: &Shaders,
                 projection_matrix: &Matrix4<f32>,
                 map_render_data: &MapRenderData) {
    unsafe {
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }

    let model = Matrix4::<f32>::identity();
    let model_view = view * model;
    let normal_matrix = {
        let inverted = model_view.try_inverse().unwrap();
        let m3x3 = inverted.fixed_slice::<nalgebra::base::U3, nalgebra::base::U3>(0, 0);
        m3x3.transpose()
    };

    shaders.ground_shader.gl_use();
    shaders.ground_shader.set_mat4("projection", &projection_matrix);
    shaders.ground_shader.set_mat4("model_view", &model_view);
    shaders.ground_shader.set_mat3("normal_matrix", &normal_matrix);

    shaders.ground_shader.set_vec3("light_dir", &map_render_data.rsw.light.direction);
    shaders.ground_shader.set_vec3("light_ambient", &map_render_data.rsw.light.ambient);
    shaders.ground_shader.set_vec3("light_diffuse", &map_render_data.rsw.light.diffuse);
    shaders.ground_shader.set_f32("light_opacity", map_render_data.rsw.light.opacity);

    shaders.ground_shader.set_vec3("in_lightWheight", &map_render_data.light_wheight);

    map_render_data.texture_atlas.bind(gl::TEXTURE0);
    shaders.ground_shader.set_int("gnd_texture_atlas", 0);

    map_render_data.tile_color_texture.bind(gl::TEXTURE1);
    shaders.ground_shader.set_int("tile_color_texture", 1);

    map_render_data.lightmap_texture.bind(gl::TEXTURE2);
    shaders.ground_shader.set_int("lightmap_texture", 2);

    shaders.ground_shader.set_int("use_tile_color", if map_render_data.use_tile_colors { 1 } else { 0 });
    shaders.ground_shader.set_int("use_lightmap", if map_render_data.use_lightmaps { 1 } else { 0 });
    shaders.ground_shader.set_int("use_lighting", if map_render_data.use_lighting { 1 } else { 0 });


    map_render_data.ground_vertex_array_obj.bind().draw(gl::TRIANGLES);

    if map_render_data.draw_models {
        shaders.model_shader.gl_use();
        shaders.model_shader.set_mat4("projection", &projection_matrix);
        shaders.model_shader.set_mat4("view", &view);
        shaders.model_shader.set_mat3("normal_matrix", &normal_matrix);
        shaders.model_shader.set_int("model_texture", 0);

        shaders.model_shader.set_vec3("light_dir", &map_render_data.rsw.light.direction);
        shaders.model_shader.set_vec3("light_ambient", &map_render_data.rsw.light.ambient);
        shaders.model_shader.set_vec3("light_diffuse", &map_render_data.rsw.light.diffuse);
        shaders.model_shader.set_f32("light_opacity", map_render_data.rsw.light.opacity);

        shaders.model_shader.set_int("use_lighting", if map_render_data.use_lighting { 1 } else { 0 });

        for (model_name, matrix) in &map_render_data.model_instances {
            shaders.model_shader.set_mat4("model", &matrix);
            let model_render_data = &map_render_data.models[&model_name];
            shaders.model_shader.set_f32("alpha", model_render_data.alpha);
            for node_render_data in &model_render_data.model {
                // TODO: optimize this
                for face_render_data in node_render_data {
                    face_render_data.texture.bind(gl::TEXTURE0);
                    face_render_data.vao.bind().draw(gl::TRIANGLES);
                }
            }
        }
    }
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
    pub ground_vertex_array_obj: VertexArray,
    pub sprite_vertex_array: VertexArray,
    pub texture_atlas: GlTexture,
    pub tile_color_texture: GlTexture,
    pub lightmap_texture: GlTexture,
    pub models: HashMap<ModelName, ModelRenderData>,
    pub model_instances: Vec<(ModelName, Matrix4<f32>)>,
    pub draw_models: bool,
}

pub struct EntityRenderData {
    pub pos: Vector3<f32>,
//    pub texture: GlTexture,
}

pub type DataForRenderingSingleNode = Vec<SameTextureNodeFaces>;

pub struct ModelRenderData {
    pub alpha: f32,
    pub model: Vec<DataForRenderingSingleNode>,
}

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
            let data_for_rendering_full_model: Vec<DataForRenderingSingleNode> = Rsm::generate_meshes_by_texture_id(
                &rsm.bounding_box,
                rsm.shade_type,
                rsm.nodes.len() == 1,
                &rsm.nodes,
                &textures,
            );
            (name.clone(), ModelRenderData {
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
    let sprite_vertex_array = VertexArray::new(&s, 4, None, vec![
        VertexAttribDefinition {
            number_of_components: 2,
            offset_of_first_element: 0,
        }, VertexAttribDefinition { // uv
            number_of_components: 2,
            offset_of_first_element: 2,
        }
    ]);

    let vertex_array = VertexArray::new(&ground.mesh, ground.mesh.len(), None, vec![
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
        ground_vertex_array_obj: vertex_array,
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