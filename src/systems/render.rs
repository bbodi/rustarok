use nalgebra::{Matrix4, Vector3, Rotation3, Point3};
use crate::video::{VertexArray, draw_lines_inefficiently, draw_circle_inefficiently};
use crate::video::VertexAttribDefinition;
use crate::components::{CameraComponent, BrowserClient, PositionComponent, PhysicsComponent, DirectionComponent, AnimatedSpriteComponent, DummyAiComponent};
use specs::prelude::*;
use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::{Shaders, MapRenderData};

// the values that should be added to the sprite direction based on the camera
// direction (the index is the camera direction, which is floor(angle/45)
pub const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];

pub struct RenderBrowserClientsSystem;

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

pub struct RenderDesktopClientSystem;

impl<'a> specs::System<'a> for RenderDesktopClientSystem {
    type SystemData = (
        specs::ReadStorage<'a, CameraComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PositionComponent>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, DirectionComponent>,
        specs::ReadStorage<'a, AnimatedSpriteComponent>,
        specs::ReadStorage<'a, DummyAiComponent>,
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
        ai_storage,
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


            for (_entity_pos, physics, dir, animated_sprite,
                ai) in (&position_storage,
                        &physics_storage,
                        &dir_storage,
                        &animated_sprite_storage,
                        &ai_storage).join() {
                system_vars.shaders.sprite_shader.gl_use();
                system_vars.shaders.sprite_shader.set_mat4("projection", &system_vars.matrices.projection);
                system_vars.shaders.sprite_shader.set_mat4("view", &view);
                system_vars.shaders.sprite_shader.set_int("model_texture", 0);
                system_vars.shaders.sprite_shader.set_f32("alpha", 1.0);
                let binded_sprite_vertex_array = system_vars.map_render_data.sprite_vertex_array.bind();

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
                    let body = system_vars.physics_world.rigid_body(physics.handle).unwrap();
                    let pos = body.position().translation.vector;
                    let mut pos = Vector3::new(pos.x, 1.0, pos.y);
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

                    binded_sprite_vertex_array.draw();
                }

//                let body = system_vars.physics_world.rigid_body(physics.handle).unwrap();
//                let pos = body.position().translation.vector;
//                let pos = Vector3::new(pos.x, 1.0, pos.y);
//                let target = Vector3::new(ai.target_pos.x, 1.0, ai.target_pos.y);
//                draw_lines_inefficiently(
//                    &system_vars.shaders.trimesh_shader,
//                    &system_vars.matrices.projection,
//                    &view,
//                    &[pos, target],
//                    &[1.0, 0.0, 1.0],
//                );
            }
        }
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
    map_render_data.ground_vertex_array.bind().draw();


    if map_render_data.draw_models {
        for (i, (model_name, matrix)) in map_render_data.model_instances.iter().enumerate() {
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


            ///
            shaders.model_shader.set_mat4("model", &matrix);
            let model_render_data = &map_render_data.models[&model_name];
            shaders.model_shader.set_f32("alpha", model_render_data.alpha);
            for node_render_data in &model_render_data.model {
                // TODO: optimize this
                for face_render_data in node_render_data {
                    face_render_data.texture.bind(gl::TEXTURE0);
                    face_render_data.vao.bind().draw();
                }
            }

            let bbox = &model_render_data.bounding_box;
            let min = bbox.min;
            let min = matrix.transform_point(&Point3::new(min.x, 0.0, min.z));
            let max = bbox.max;
            let max = matrix.transform_point(&Point3::new(max.x, 0.0, max.z));

//            draw_lines_inefficiently(
//                &shaders.trimesh_shader,
//                &projection_matrix,
//                &view,
//                &[min.coords, max.coords],
//                &[1.0, 0.0, 0.0],
//            );

            let r: f32 = nalgebra::distance(&min, &max) / 2.0;
            let center = bbox.center;
            let center = matrix.transform_point(&Point3::new(center.x, center.y, center.z));
            draw_circle_inefficiently(
                &shaders.trimesh_shader,
                &projection_matrix,
                &view,
                &center.coords,
                r,
                &[1.0, 0.0, 0.0],
            );
        }
    }
}


pub struct PhysicsDebugDrawingSystem {
    capsule_vertex_array: VertexArray,
}

impl PhysicsDebugDrawingSystem {
    pub fn new() -> PhysicsDebugDrawingSystem {
        let mut capsule_mesh = ncollide2d::procedural::circle(
            &2.0f32,
            32,
        );

        let coords = capsule_mesh.coords();
        let capsule_vertex_array = VertexArray::new(
            gl::LINE_LOOP,
            coords,
            coords.len(),
            None,
            vec![
                VertexAttribDefinition {
                    number_of_components: 2,
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
                let pos = Vector3::new(pos.x, 1.0, pos.y);
                matrix.prepend_translation_mut(&pos);
                let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::x()), std::f32::consts::FRAC_PI_2).to_homogeneous();
                matrix = matrix * rotation;

                system_vars.shaders.trimesh_shader.gl_use();
                system_vars.shaders.trimesh_shader.set_mat4("projection", &system_vars.matrices.projection);
                system_vars.shaders.trimesh_shader.set_mat4("view", &view);
                system_vars.shaders.trimesh_shader.set_f32("alpha", 1.0);
                system_vars.shaders.trimesh_shader.set_mat4("model", &matrix);
                system_vars.shaders.trimesh_shader.set_vec3("color", &[1.0, 0.0, 1.0]);
                self.capsule_vertex_array.bind().draw();
            }
        }
    }
}


pub struct RenderStreamingSystem;

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