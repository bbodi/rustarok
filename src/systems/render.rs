use nalgebra::{Matrix4, Vector3, Rotation3, Point3, Vector2, Point2, Matrix3, Vector4};
use crate::video::{VertexArray, draw_lines_inefficiently, draw_circle_inefficiently, draw_lines_inefficiently2, VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::video::VertexAttribDefinition;
use specs::prelude::*;
use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::{Shaders, MapRenderData, SpriteResource, Tick, PhysicsWorld, TICKS_PER_SECOND, ElapsedTime};
use std::collections::HashMap;
use crate::cam::Camera;
use std::cmp::max;
use crate::components::controller::ControllerComponent;
use crate::components::{BrowserClient, FlyingNumberComponent};
use crate::components::char::{PhysicsComponent, PlayerSpriteComponent, MonsterSpriteComponent, CharacterStateComponent, ComponentRadius, SpriteBoundingRect, SpriteRenderDescriptor};
use crate::components::skill::{PushBackWallSkill, SkillManifestationComponent};
use ncollide2d::shape::Shape;
use crate::consts::{JobId, MonsterId};

// the values that should be added to the sprite direction based on the camera
// direction (the index is the camera direction, which is floor(angle/45)
pub const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];

pub const ONE_SPRITE_PIXEL_SIZE_IN_3D: f32 = (1.0 / 175.0) * 5.0;

pub struct OpenGlInitializerFor3D;

impl<'a> specs::System<'a> for OpenGlInitializerFor3D {
    type SystemData = (
        specs::ReadStorage<'a, ControllerComponent>,
        specs::WriteStorage<'a, BrowserClient>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(&mut self, (
        controller_storage,
        mut browser_client_storage,
        system_vars,
    ): Self::SystemData) {
        unsafe {
            gl::Enable(gl::DEPTH_TEST); // depth test is disabled for damages, here we turn it back
        }
        for (controller, browser) in (&controller_storage, &mut browser_client_storage).join() {
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
                gl::ReadPixels(0, 0, VIDEO_WIDTH as i32, VIDEO_HEIGHT as i32,
                               gl::RGBA,
                               gl::UNSIGNED_BYTE, browser.offscreen.as_mut_ptr() as *mut gl::types::GLvoid);
            }
        }
    }
}

pub struct RenderDesktopClientSystem {
    rectangle_vao: VertexArray,
}

impl RenderDesktopClientSystem {
    pub fn new() -> RenderDesktopClientSystem {
        let s: Vec<[f32; 2]> = vec![
            [0.0, 1.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 0.0]
        ];

        RenderDesktopClientSystem {
            rectangle_vao: VertexArray::new(
                gl::TRIANGLE_STRIP,
                &s, 4, None, vec![
                    VertexAttribDefinition {
                        number_of_components: 2,
                        offset_of_first_element: 0,
                    }
                ]),
        }
    }
}

impl<'a> specs::System<'a> for RenderDesktopClientSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, PlayerSpriteComponent>,
        specs::ReadStorage<'a, MonsterSpriteComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::ReadExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteStorage<'a, SkillManifestationComponent>, // TODO remove me
    );

    fn run(&mut self, (
        entities,
        input_storage,
        browser_client_storage,
        physics_storage,
        player_sprite_storage,
        monster_sprite_storage,
        mut char_state_storage,
        system_vars,
        physics_world,
        mut system_benchmark,
        mut skill_storage,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("RenderDesktopClientSystem");
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        for (controller, _not_browser) in (&input_storage, !&browser_client_storage).join() {
            // Draw players
            for (entity, physics, animated_sprite, char_state) in (&entities, &physics_storage,
                                                                   &player_sprite_storage,
                                                                   &mut char_state_storage).join() {
                let pos = physics.pos(&physics_world);
                let tick = system_vars.tick;
                let body_res = {
                    let sprites = &system_vars.sprites.character_sprites;
                    &sprites[&animated_sprite.job_id][animated_sprite.sex as usize]
                };
                let head_res = {
                    let sprites = &system_vars.sprites.head_sprites;
                    &sprites[animated_sprite.sex as usize][animated_sprite.head_index]
                };
                if system_vars.entity_below_cursor.filter(|it| *it == entity).is_some() {
                    let (pos_offset, _body_bounding_rect) = render_sprite(&system_vars,
                                                                         &animated_sprite.descr,
                                                                         body_res,
                                                                         &system_vars.matrices.view,
                                                                         Some(controller.yaw),
                                                                         &pos,
                                                                         [0, 0],
                                                                         true,
                                                                         1.1,
                                                                         &[0.0, 0.0, 1.0, 0.4]);

                    let (_head_pos_offset, _head_bounding_rect) = render_sprite(&system_vars,
                                                                                &animated_sprite.descr,
                                                                                head_res,
                                                                                &system_vars.matrices.view,
                                                                                Some(controller.yaw),
                                                                                &pos,
                                                                                pos_offset,
                                                                                false,
                                                                                1.1,
                                                                                &[0.0, 0.0, 1.0, 0.5]);
                }

                // todo: kell a pos_offset még mindig? (bounding rect)
                let (pos_offset, body_bounding_rect) = render_sprite(&system_vars,
                                                                     &animated_sprite.descr,
                                                                     body_res,
                                                                     &system_vars.matrices.view,
                                                                     Some(controller.yaw),
                                                                     &pos,
                                                                     [0, 0],
                                                                     true,
                                                                     1.0,
                                                                     &[1.0, 1.0, 1.0, 1.0]);
                let (head_pos_offset, head_bounding_rect) = render_sprite(&system_vars,
                                                                          &animated_sprite.descr,
                                                                          head_res,
                                                                          &system_vars.matrices.view,
                                                                          Some(controller.yaw),
                                                                          &pos,
                                                                          pos_offset,
                                                                          false,
                                                                          1.0,
                                                                          &[1.0, 1.0, 1.0, 1.0]);

                char_state.bounding_rect_2d = body_bounding_rect;
                char_state.bounding_rect_2d.merge(&head_bounding_rect);

                // draw health bars etc
                let shader = system_vars.shaders.trimesh2d_shader.gl_use();
                shader.set_mat4("projection", &system_vars.matrices.ortho);
                let vao = self.rectangle_vao.bind();
                let draw_rect = |x: i32, y: i32, w: i32, h: i32, color: &[f32;4]| {
                    let mut matrix = Matrix4::<f32>::identity();
                    let spr_x = char_state.bounding_rect_2d.bottom_left[0];
                    let spr_w = char_state.bounding_rect_2d.top_right[0] - char_state.bounding_rect_2d.bottom_left[0];
                    let bar_x = spr_x as f32 + (spr_w as f32/2.0) - (130.0/2.0);
                    let pos = Vector3::new(
                        bar_x + x as f32,
                        char_state.bounding_rect_2d.top_right[1] as f32 - 40.0 + y as f32,
                        0.0,
                    );
                    matrix.prepend_translation_mut(&pos);
                    shader.set_mat4("model", &matrix);
                    shader.set_vec4("color", color);
                    shader.set_vec2("size", &[w as f32, h as f32]);
                    vao.draw();
                };
                draw_rect(0, 0, 130, 19, &[0.0, 0.0, 0.0, 1.0]); // black background
                draw_rect(1, 1, 128, 17, &[0.44, 0.49, 0.56, 1.0]); // grey inner

                draw_rect(2, 2, 126, 10, &[0.0, 0.0, 0.0, 1.0]); // black health background
                draw_rect(3, 3, 124, 8, &[0.29, 0.80, 0.11, 1.0]); // green health bar

                draw_rect(2, 12, 126, 5, &[0.0, 0.0, 0.0, 1.0]); // black mana background
                draw_rect(3, 13, 100, 3, &[0.23, 0.79, 0.88, 1.0]); // blue mana bar
            }
            // Draw monsters
            for (entity, physics, animated_sprite, char_state) in (&entities, &physics_storage,
                                                                   &monster_sprite_storage,
                                                                   &mut char_state_storage).join() {
                let pos = physics.pos(&physics_world);
                let tick = system_vars.tick;
                let body_res = {
                    let sprites = &system_vars.sprites.monster_sprites;
                    &sprites[&animated_sprite.monster_id]
                };
                if system_vars.entity_below_cursor.filter(|it| *it == entity).is_some() {
                    let (pos_offset, bounding_rect) = render_sprite(&system_vars,
                                                                    &animated_sprite.descr,
                                                                    body_res,
                                                                    &system_vars.matrices.view,
                                                                    Some(controller.yaw),
                                                                    &pos,
                                                                    [0, 0],
                                                                    true,
                                                                    1.1,
                                                                    &[0.0, 0.0, 1.0, 0.5]);
                }
                let (pos_offset, bounding_rect) = render_sprite(&system_vars,
                                                                &animated_sprite.descr,
                                                                body_res,
                                                                &system_vars.matrices.view,
                                                                Some(controller.yaw),
                                                                &pos,
                                                                [0, 0],
                                                                true,
                                                                1.0,
                                                                &[1.0, 1.0, 1.0, 1.0]);
                char_state.bounding_rect_2d = bounding_rect;
            }

            let physics = physics_storage.get(controller.char).unwrap();
            let char_pos = physics.pos(&physics_world);
            render_client(
                &char_pos,
                &controller.camera,
                &system_vars.matrices.view,
                &system_vars.shaders,
                &system_vars.matrices.projection,
                &system_vars.map_render_data,
            );

            for (skill) in (&skill_storage).join() {
                skill.render(&system_vars);
            }
        }
    }
}

fn set_spherical_billboard(model_view: &mut Matrix4<f32>) {
    model_view[0] = 1.0;
    model_view[1] = 0.0;
    model_view[2] = 0.0;
    model_view[4] = 0.0;
    model_view[5] = 1.0;
    model_view[6] = 0.0;
    model_view[8] = 0.0;
    model_view[9] = 0.0;
    model_view[10] = 1.0;
}

pub fn render_sprite(system_vars: &SystemVariables,
                     animation: &SpriteRenderDescriptor,
                     sprite_res: &SpriteResource,
                     view: &Matrix4<f32>,
                     yaw: Option<f32>,
                     pos: &Vector2<f32>,
                     pos_offset: [i32; 2],
                     is_main: bool,
                     size_multiplier: f32,
                     color: &[f32; 4],
) -> ([i32; 2], SpriteBoundingRect) {
    let shader = system_vars.shaders.player_shader.gl_use();
    shader.set_mat4("projection", &system_vars.matrices.projection);
    shader.set_mat4("view", &view);
    shader.set_int("model_texture", 0);
    let binded_sprite_vertex_array = system_vars.map_render_data.sprite_vertex_array.bind();

    // draw layer

    let idx = if let Some(yaw) = yaw {
        let cam_dir = (((yaw / 45.0) + 0.5) as usize) % 8;
        animation.action_index + (animation.direction + DIRECTION_TABLE[cam_dir]) % 8
    } else {
        0
    };

    let frame_count = sprite_res.action.actions[idx].frames.len();
    let mut time_needed_for_one_frame = if let Some(duration) = animation.forced_duration {
        duration.div(frame_count as f32)
    } else {
        sprite_res.action.actions[idx].delay as f32 / 1000.0
    };
    time_needed_for_one_frame = if time_needed_for_one_frame == 0.0 { 0.1 } else { time_needed_for_one_frame };
    let elapsed_time = system_vars.time.elapsed_since(&animation.animation_started);
    let frame_index = ((elapsed_time.div(time_needed_for_one_frame)) as usize % frame_count) as usize;
    let action_anim = &sprite_res.action.actions[idx].frames[frame_index];

    // TODO: refactor: ugly, valahol csak 1 layert kell irajzolni (player), valahol többet is (effektek)
    let mut width = 0.0;
    let mut height = 0.0;
    let mut offset = [0.0, 0.0];
    let matrix = {
        let mut matrix = Matrix4::<f32>::identity();
        let pos = Vector3::new(pos.x, 0.0, pos.y); // y was 1.0
        matrix.prepend_translation_mut(&pos);
        shader.set_mat4("model", &matrix);
        matrix
    };
    for layer in action_anim.layers.iter() {
        if idx == 40 {
            dbg!(layer.sprite_frame_index);
        }
        if layer.sprite_frame_index < 0 {
            continue;
        }
        let sprite_texture = &sprite_res.textures[layer.sprite_frame_index as usize];

        width = sprite_texture.original_width as f32 * layer.scale[0] * size_multiplier;
        height = sprite_texture.original_height as f32 * layer.scale[1] * size_multiplier;
        sprite_texture.texture.bind(gl::TEXTURE0);

        offset = if !action_anim.positions.is_empty() && !is_main {
            [
                (pos_offset[0] - action_anim.positions[0][0]) as f32,
                (pos_offset[1] - action_anim.positions[0][1]) as f32
            ]
        } else {
            [0.0, 0.0]
        };
        offset = [layer.pos[0] as f32 + offset[0], layer.pos[1] as f32 + offset[1]];
        offset = [
            offset[0] as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D,
            offset[1] as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D - 0.5
        ];
        shader.set_vec2("offset", &offset);

        width = width as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D;
        width = if layer.is_mirror { -width } else { width };
        height = height as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D;
        shader.set_vec2("size", &[width, height]);

        let mut color = color.clone();
        for i in 0..4 {
            color[i] *= layer.color[i];
        }
        shader.set_vec4("color", &color);

        binded_sprite_vertex_array.draw();
    }
    let anim_pos = action_anim.positions.get(0).map(|it| it.clone()).unwrap_or([0, 0]);

    let size = [width.abs(), height];
    let bb = project_to_screen(&size, &offset, view * matrix, &system_vars.matrices.projection);
    return ([
                (anim_pos[0] as f32 * size_multiplier) as i32,
                (anim_pos[1] as f32 * size_multiplier) as i32
            ], bb);
}

fn project_to_screen(size: &[f32; 2], offset: &[f32; 2], mut model_view: Matrix4<f32>, projection: &Matrix4<f32>) -> SpriteBoundingRect {
    let mut top_right = Vector4::new(0.5 * size[0], 0.5 * size[1], 0.0, 1.0);
    top_right.x += offset[0];
    top_right.y -= offset[1]; // itt régen + 0.5

    let mut bottom_left = Vector4::new(-0.5 * size[0], -0.5 * size[1], 0.0, 1.0);
    bottom_left.x += offset[0];
    bottom_left.y -= offset[1]; // itt régen + 0.5

    set_spherical_billboard(&mut model_view);
    fn sh(v: Vector4<f32>) -> [i32; 2] {
        let s = if v[3] == 0.0 { 1.0 } else { 1.0 / v[3] };
        [
            ((v[0] * s / 2.0 + 0.5) * VIDEO_WIDTH as f32) as i32,
            VIDEO_HEIGHT as i32 - ((v[1] * s / 2.0 + 0.5) * VIDEO_HEIGHT as f32) as i32
        ]
    }
    let bottom_left = sh(projection * model_view * bottom_left);
    let top_right = sh(projection * model_view * top_right);
    return SpriteBoundingRect {
        bottom_left,
        top_right,
    };
}

fn render_client(char_pos: &Vector2<f32>,
                 camera: &Camera,
                 view: &Matrix4<f32>,
                 shaders: &Shaders,
                 projection_matrix: &Matrix4<f32>,
                 map_render_data: &MapRenderData) {
    let model = Matrix4::<f32>::identity();
    let model_view = view * model;
    let normal_matrix = {
        let inverted = model_view.try_inverse().unwrap();
        let m3x3 = inverted.fixed_slice::<nalgebra::base::U3, nalgebra::base::U3>(0, 0);
        m3x3.transpose()
    };

    render_ground(shaders, projection_matrix, map_render_data, &model_view, &normal_matrix);

//    shaders.trimesh_shader.gl_use();
//    shaders.trimesh_shader.set_mat4("projection", &projection_matrix);
//    shaders.trimesh_shader.set_mat4("view", view);
//    shaders.trimesh_shader.set_vec4("color", &[0.0, 1.0, 0.0, 0.5]);
//    shaders.trimesh_shader.set_mat4("model", &Matrix4::identity());
//    map_render_data.ground_walkability_mesh.bind().draw();
//
//    shaders.trimesh_shader.set_vec4("color", &[1.0, 0.0, 0.0, 0.5]);
//    map_render_data.ground_walkability_mesh2.bind().draw();
//
//    shaders.trimesh_shader.set_vec4("color", &[0.0, 0.0, 1.0, 0.5]);
//    map_render_data.ground_walkability_mesh3.bind().draw();

    let shader = shaders.model_shader.gl_use();
    shader.set_mat4("projection", &projection_matrix);
    shader.set_mat4("view", &view);
    shader.set_mat3("normal_matrix", &normal_matrix);
    shader.set_int("model_texture", 0);

    shader.set_vec3("light_dir", &map_render_data.rsw.light.direction);
    shader.set_vec3("light_ambient", &map_render_data.rsw.light.ambient);
    shader.set_vec3("light_diffuse", &map_render_data.rsw.light.diffuse);
    shader.set_f32("light_opacity", map_render_data.rsw.light.opacity);

    shader.set_int("use_lighting", if map_render_data.use_lighting { 1 } else { 0 });

    // cam area is [-20;20] width and [70;5] height
    if map_render_data.draw_models {
        for (i, (model_name, matrix)) in map_render_data.model_instances.iter().enumerate() {
            let model_render_data = &map_render_data.models[&model_name];
            // TODO: before transformation, max and min is reversed
//            min: Point {
//                coords: Matrix {
//                    data: [
//                        -32.90765,
//                        -70.0698,
//                        -30.87375,
//                    ],
//                },
//            },
//            max: Point {
//                coords: Matrix {
//                    data: [
//                        32.90765,
//                        0.0,
//                        30.87375,
//                    ],
//                },
//            },
            let min = matrix.transform_point(&model_render_data.bounding_box.min);
            let max = matrix.transform_point(&model_render_data.bounding_box.max);
            let cam_pos = camera.pos();
            if ((max.x < cam_pos.x - 40.0 || max.x > cam_pos.x + 40.0) &&
                (min.x < cam_pos.x - 40.0 || min.x > cam_pos.x + 40.0)) ||
                ((max.z < cam_pos.z - 70.0 || max.z > cam_pos.z + 5.0) &&
                    (min.z < cam_pos.z - 70.0 || min.z > cam_pos.z + 5.0))
            {
                continue;
            }
            shader.set_mat4("model", &matrix);
            let alpha = if
            ((max.x > cam_pos.x - 10.0 && max.x < cam_pos.x + 10.0) ||
                (min.x > cam_pos.x - 10.0 && min.x < cam_pos.x + 10.0))
                && char_pos.y <= max.z // character is behind
                && min.y > 5.0 {
                0.3
            } else {
                model_render_data.alpha
            };
            shader.set_f32("alpha", alpha);
            for node_render_data in &model_render_data.model {
                // TODO: optimize this
                for face_render_data in node_render_data {
                    face_render_data.texture.bind(gl::TEXTURE0);
                    face_render_data.vao.bind().draw();
                }
            }
        }
    }
}

fn render_ground(shaders: &Shaders,
                 projection_matrix: &Matrix4<f32>,
                 map_render_data: &MapRenderData,
                 model_view: &Matrix4<f32>,
                 normal_matrix: &Matrix3<f32>) {
    let shader = shaders.ground_shader.gl_use();
    shader.set_mat4("projection", &projection_matrix);
    shader.set_mat4("model_view", &model_view);
    shader.set_mat3("normal_matrix", &normal_matrix);
    shader.set_vec3("light_dir", &map_render_data.rsw.light.direction);
    shader.set_vec3("light_ambient", &map_render_data.rsw.light.ambient);
    shader.set_vec3("light_diffuse", &map_render_data.rsw.light.diffuse);
    shader.set_f32("light_opacity", map_render_data.rsw.light.opacity);
    shader.set_vec3("in_lightWheight", &map_render_data.light_wheight);
    map_render_data.texture_atlas.bind(gl::TEXTURE0);
    shader.set_int("gnd_texture_atlas", 0);
    map_render_data.tile_color_texture.bind(gl::TEXTURE1);
    shader.set_int("tile_color_texture", 1);
    map_render_data.lightmap_texture.bind(gl::TEXTURE2);
    shader.set_int("lightmap_texture", 2);
    shader.set_int("use_tile_color", if map_render_data.use_tile_colors { 1 } else { 0 });
    shader.set_int("use_lightmap", if map_render_data.use_lightmaps { 1 } else { 0 });
    shader.set_int("use_lighting", if map_render_data.use_lighting { 1 } else { 0 });
    map_render_data.ground_vertex_array.bind().draw();
}


pub struct PhysicsDebugDrawingSystem {
    capsule_vertex_arrays: HashMap<ComponentRadius, VertexArray>, // radius to vertexArray
}

impl PhysicsDebugDrawingSystem {
    pub fn new() -> PhysicsDebugDrawingSystem {
        let capsule_vertex_arrays: HashMap<ComponentRadius, VertexArray> = [1, 2, 3, 4].iter().map(|radius| {
            let mut capsule_mesh = ncollide2d::procedural::circle(
                &(*radius as f32 * 0.5 * 2.0),
                32,
            );

            let coords = capsule_mesh.coords();
            (ComponentRadius(*radius), VertexArray::new(
                gl::LINE_LOOP,
                coords,
                coords.len(),
                None,
                vec![
                    VertexAttribDefinition {
                        number_of_components: 2,
                        offset_of_first_element: 0,
                    }
                ])
            )
        }).collect();
        PhysicsDebugDrawingSystem {
            capsule_vertex_arrays,
        }
    }
}

impl<'a> specs::System<'a> for PhysicsDebugDrawingSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::ReadExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        entities,
        controller_storage,
        browser_client_storage,
        physics_storage,
        system_vars,
        physics_world,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("PhysicsDebugDrawingSystem");
        for (controller, _not_browser) in (&controller_storage, !&browser_client_storage).join() {
            for physics in (&physics_storage).join() {
                let mut matrix = Matrix4::<f32>::identity();
                let pos = physics.pos(&physics_world);
                let pos = Vector3::new(pos.x, 1.0, pos.y);
                matrix.prepend_translation_mut(&pos);
                let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::x()), std::f32::consts::FRAC_PI_2).to_homogeneous();
                matrix = matrix * rotation;

                let shader = system_vars.shaders.trimesh_shader.gl_use();
                shader.set_mat4("projection", &system_vars.matrices.projection);
                shader.set_mat4("view", &system_vars.matrices.view);
                shader.set_f32("alpha", 1.0);
                shader.set_mat4("model", &matrix);
                shader.set_vec4("color", &[1.0, 0.0, 1.0, 1.0]);
                self.capsule_vertex_arrays[&physics.radius].bind().draw();
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


pub struct DamageRenderSystem {
    single_digit_u_coord: f32,
    texture_u_coords: [f32; 10],
}

impl DamageRenderSystem {
    pub fn new() -> DamageRenderSystem {
        let single_digit_width = 10.0;
        let texture_width = single_digit_width * 10.0;
        let single_digit_u_coord = single_digit_width / texture_width;
        DamageRenderSystem {
            single_digit_u_coord,
            texture_u_coords: [
                single_digit_u_coord * 0.0,
                single_digit_u_coord * 1.0,
                single_digit_u_coord * 2.0,
                single_digit_u_coord * 3.0,
                single_digit_u_coord * 4.0,
                single_digit_u_coord * 5.0,
                single_digit_u_coord * 6.0,
                single_digit_u_coord * 7.0,
                single_digit_u_coord * 8.0,
                single_digit_u_coord * 9.0,
            ],
        }
    }

    fn get_digits(n: u32) -> Vec<u8> {
        let mut digits = Vec::new();
        let mut n = n;
        while n > 9 {
            digits.push((n % 10) as u8);
            n = n / 10;
        }
        digits.push(n as u8);
        digits.reverse();
        digits
    }
}

impl<'a> specs::System<'a> for DamageRenderSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, FlyingNumberComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );
    fn run(&mut self, (
        entities,
        numbers,
        system_vars,
        mut system_benchmark,
        updater,
    ): Self::SystemData) {
        unsafe {
            gl::Disable(gl::DEPTH_TEST);
        }
        let stopwatch = system_benchmark.start_measurement("DamageRenderSystem");

        for (entity, number) in (&entities, &numbers).join() {
            let digits = DamageRenderSystem::get_digits(number.value);
            // create vbo based on the numbers
            let mut x = 0.0;
            let mut vertices = vec![];
            digits.iter().for_each(|&digit| {
                let digit = digit as usize;
                vertices.push([x - 0.5, 0.5, self.texture_u_coords[digit], 0.0]);
                vertices.push([x + 0.5, 0.5, self.texture_u_coords[digit] + self.single_digit_u_coord, 0.0]);
                vertices.push([x - 0.5, -0.5, self.texture_u_coords[digit], 1.0]);
                vertices.push([x + 0.5, 0.5, self.texture_u_coords[digit] + self.single_digit_u_coord, 0.0]);
                vertices.push([x - 0.5, -0.5, self.texture_u_coords[digit], 1.0]);
                vertices.push([x + 0.5, -0.5, self.texture_u_coords[digit] + self.single_digit_u_coord, 1.0]);
                x += 1.0;
            });
            let vertex_array = VertexArray::new(
                gl::TRIANGLES,
                &vertices, vertices.len(), None, vec![
                    VertexAttribDefinition {
                        number_of_components: 2,
                        offset_of_first_element: 0,
                    }, VertexAttribDefinition { // uv
                        number_of_components: 2,
                        offset_of_first_element: 2,
                    }
                ]);

            system_vars.sprites.numbers.bind(gl::TEXTURE0);
            let shader = system_vars.shaders.sprite_shader.gl_use();
            let mut matrix = Matrix4::<f32>::identity();
            let mut pos = Vector3::new(number.start_pos.x, 1.0, number.start_pos.y);

            pos.y += 20.0 * ((system_vars.tick.0 - number.start_tick.0) as f32 / number.duration as f32);
            pos.z -= 10.0 * ((system_vars.tick.0 - number.start_tick.0) as f32 / number.duration as f32);
            pos.x += 10.0 * ((system_vars.tick.0 - number.start_tick.0) as f32 / number.duration as f32);
            matrix.prepend_translation_mut(&pos);
            shader.set_mat4("model", &matrix);
            shader.set_vec3("color", &number.color);

            shader.set_vec2("size", &[
                1.0,
                1.0
            ]);
            shader.set_mat4("projection", &system_vars.matrices.projection);
            shader.set_mat4("view", &system_vars.matrices.view);
            shader.set_int("model_texture", 0);
            shader.set_f32("alpha", 1.0);

            vertex_array.bind().draw();

            if system_vars.tick.0 > (number.start_tick.0 + number.duration as u64) {
                updater.remove::<FlyingNumberComponent>(entity);
            }
        }
    }
}