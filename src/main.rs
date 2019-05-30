extern crate sdl2;
extern crate gl;
extern crate nalgebra;
extern crate nalgebra_glm;
extern crate encoding;
#[macro_use]
extern crate imgui;
extern crate imgui_sdl2;
extern crate imgui_opengl_renderer;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::string::FromUtf8Error;
use std::path::Path;
use std::io::Cursor;
use crate::common::BinaryReader;
use crate::rsw::{Rsw, GroundData};
use crate::gnd::{Gnd, MeshVertex};
use crate::gat::Gat;
use std::ffi::{CString, CStr};

use imgui::{ImGuiCond, ImString, ImStr};
use nalgebra::{Vector3, Matrix4, Point3, Matrix};
use crate::opengl::{Shader, Program, VertexArray, VertexAttribDefinition, GlTexture};
use std::time::Duration;

// guild_vs4.rsw

mod common;
mod opengl;
mod gat;
mod rsw;
mod gnd;


fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let gl_attr = video_subsystem.gl_attr();

    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 5);
    let window = video_subsystem
        .window("Game", 900, 700)
        .opengl() // add opengl flag
        .allow_highdpi()
        .resizable()
        .build()
        .unwrap();

    let gl_context = window.gl_create_context().unwrap();
    let gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    unsafe {
        gl::Viewport(0, 0, 900, 700); // set viewport
        gl::ClearColor(0.3, 0.3, 0.5, 1.0);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
//
        gl::Enable(gl::BLEND);
        // ezzel nem látszóüdik semmi
//        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }


    let vert_shader = Shader::from_source(
        include_str!("triangle.vert"),
        gl::VERTEX_SHADER,
    ).unwrap();

    let frag_shader = Shader::from_source(
        include_str!("triangle.frag"),
        gl::FRAGMENT_SHADER,
    ).unwrap();

    let shader_program = Program::from_shaders(
        &[vert_shader, frag_shader]
    ).unwrap();

    shader_program.gl_use();


    let (mut ground, mut vao, mut texture_atlas, mut tile_color_texture) = load_map("new_zone01");
//    let xyz = VertexArray::new(&vec![
//        -0.5f32, 0.0, -0.5, // x
//        0.0, 0.0, -0.5, // center
//        0.0, 0.0, -1.0, // depth
//        0.0, 0.0, -0.5, // center
//        0.0, 0.5, -0.5,   // y
//        0.0, 0.0, -0.5, // center
//    ], &[VertexAttribDefinition {
//        number_of_components: 3,
//        offset_of_first_element: 0,
//    }]);

    let mut imgui = imgui::ImGui::init();
    imgui.set_ini_filename(None);
    let video = sdl_context.video().unwrap();
    let mut imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui);

    let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, |s| video.gl_get_proc_address(s) as _);

    let mut event_pump = sdl_context.event_pump().unwrap();

    let my_str = ImString::new("shitaka");

    let mut camera_pos = Point3::<f32>::new(0.0, 0.0, 3.0);
    let mut camera_front = Vector3::<f32>::new(0.0, 0.0, -1.0);
    let world_up = Vector3::<f32>::new(0.0, 1.0, 0.0);
    let mut camera_up = world_up;
    let mut camera_right = camera_front.cross(&camera_up).normalize();

    let mut last_mouse_x = 400;
    let mut last_mouse_y = 300;
    let mut mouse_down = false;
    let mut yaw = -90f32;
    let mut pitch = 0f32;

    let mut map_name_filter = ImString::new("prontera");
    let all_map_names = std::fs::read_dir("d:\\Games\\TalonRO\\grf\\data").unwrap().map(|entry| {
        let dir_entry = entry.unwrap();
        if dir_entry.file_name().into_string().unwrap().ends_with("rsw") {
            let mut sstr = dir_entry.file_name().into_string().unwrap();
            let len = sstr.len();
            sstr.truncate(len - 4); // remove extension
            Some(sstr)
        } else { None }
    }).filter_map(|x| x).collect::<Vec<String>>();

    let proj = Matrix4::new_perspective(std::f32::consts::FRAC_PI_4, 900f32 / 700f32, 0.1f32, 1000.0f32);

    'running: loop {
        let view = Matrix4::look_at_rh(&camera_pos, &(camera_pos + camera_front), &camera_up);
        let camera_speed = 2f32;

        let model = Matrix4::<f32>::identity();

        shader_program.set_mat4("projection", &proj);
        shader_program.set_mat4("view", &view);
        shader_program.set_mat4("model", &model);

        use sdl2::event::Event;
        use sdl2::keyboard::Keycode;
        for event in event_pump.poll_iter() {
            imgui_sdl2.handle_event(&mut imgui, &event);
            if imgui_sdl2.ignore_event(&event) { continue; }

            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                }
                Event::MouseButtonDown { .. } => {
                    mouse_down = true;
                }
                Event::MouseButtonUp { .. } => {
                    mouse_down = false;
                }
                Event::MouseMotion {
                    timestamp,
                    window_id,
                    which,
                    mousestate,
                    x,
                    y,
                    xrel,
                    yrel
                } => {
                    if mouse_down {
                        let x_offset = x - last_mouse_x;
                        let y_offset = last_mouse_y - y; // reversed since y-coordinates go from bottom to top
                        yaw += x_offset as f32;
                        pitch += y_offset as f32;
                        if pitch > 89.0 {
                            pitch = 89.0;
                        }
                        if pitch < -89.0 {
                            pitch = -89.0;
                        }
                        camera_front = Vector3::<f32>::new(
                            pitch.to_radians().cos() * yaw.to_radians().cos(),
                            pitch.to_radians().sin(),
                            pitch.to_radians().cos() * yaw.to_radians().sin(),
                        ).normalize();

                        camera_right = camera_front.cross(&world_up).normalize();
                        camera_up = camera_right.cross(&camera_front).normalize();
                    }
                    last_mouse_x = x;
                    last_mouse_y = y;
                }
                Event::KeyDown { keycode, .. } => {
                    if let Some(Keycode::W) = keycode {
                        camera_pos += camera_speed * camera_front;
                    } else if let Some(Keycode::S) = keycode {
                        camera_pos -= camera_speed * camera_front;
                    }
                    if let Some(Keycode::A) = keycode {
                        camera_pos -= camera_front.cross(&camera_up).normalize() * camera_speed;
                    } else if let Some(Keycode::D) = keycode {
                        camera_pos += camera_front.cross(&camera_up).normalize() * camera_speed;
                    }
                }
                _ => {}
            }
        }

        let ui = imgui_sdl2.frame(&window, &mut imgui, &event_pump.mouse_state());

        extern crate sublime_fuzzy;
        ui.window(im_str!("Maps: {},{},{}", camera_pos.x, camera_pos.y, camera_pos.z))
            .size((300.0, 500.0), ImGuiCond::FirstUseEver)
            .build(|| {
                let map_name_filter_clone = map_name_filter.clone();
                let mut filtered_map_names = all_map_names.iter()
                    .filter(|map_name| {
                        let matc = sublime_fuzzy::best_match(map_name_filter_clone.to_str(), map_name);
                        matc.is_some()
                    });
                if ui.input_text(im_str!("Map name:"), &mut map_name_filter)
                    .enter_returns_true(true)
                    .build() {
                    if let Some(map_name) = filtered_map_names.next() {
                        let (g, v, a, tct) = load_map(map_name);
                        ground = g;
                        vao = v;
                        texture_atlas = a;
                        tile_color_texture = tct
                    }
                }
                filtered_map_names
                    .for_each(|map_name| {
                        if ui.small_button(&ImString::new(map_name.as_str())) {
                            let (g, v, a, tct) = load_map(map_name);
                            ground = g;
                            vao = v;
                            texture_atlas = a;
                            tile_color_texture = tct;
                        }
                    });
            });

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        texture_atlas.bind(gl::TEXTURE0);
        shader_program.set_int("gnd_texture_atlas", 0);

        tile_color_texture.bind(gl::TEXTURE1);
        shader_program.set_int("tile_color_texture", 1);

        unsafe {
            vao.bind();
            gl::DrawArrays(
                gl::TRIANGLES, // mode
                0, // starting index in the enabled arrays
                (ground.mesh.len()) as i32, // number of indices to be rendered
            );
        }

//        unsafe {
//            xyz.bind();
//            gl::DrawArrays(
//                gl::LINES, // mode
//                0, // starting index in the enabled arrays
//                6, // number of indices to be rendered
//            );
//        }

        renderer.render(ui);

        window.gl_swap_window();
        std::thread::sleep(Duration::from_millis(30))
    }
}

fn load_map(map_name: &str) -> (Gnd, VertexArray, GlTexture, GlTexture) {
    let world = Rsw::load(BinaryReader::new(format!("d:\\Games\\TalonRO\\grf\\data\\{}.rsw", map_name)));
    let altitude = Gat::load(BinaryReader::new(format!("d:\\Games\\TalonRO\\grf\\data\\{}.gat", map_name)));
    let mut ground = Gnd::load(BinaryReader::new(format!("d:\\Games\\TalonRO\\grf\\data\\{}.gnd", map_name)),
                           world.water.level,
                           world.water.wave_height);


    let texture_atlas = Gnd::create_gl_texture_atlas(&ground.texture_names);
    let tile_color_texture = Gnd::create_tile_color_texture(
        &mut ground.tiles_color_image,
        ground.width, ground.height,
    );
    dbg!(ground.mesh.len());
    let vertex_array = VertexArray::new(&ground.mesh, &[
        VertexAttribDefinition {
            number_of_components: 3,
            offset_of_first_element: 0,
        }, VertexAttribDefinition { // texcoords
            number_of_components: 2,
            offset_of_first_element: 6,
        }, VertexAttribDefinition { // tile color coordinate
            number_of_components: 2,
            offset_of_first_element: 10,
        }
    ]);
    (ground, vertex_array, texture_atlas, tile_color_texture)
}