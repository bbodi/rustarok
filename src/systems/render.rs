use nalgebra::{Matrix4, Vector3, Rotation3, Vector2, Matrix3, Vector4};
use crate::video::{VertexArray, VIDEO_HEIGHT, VIDEO_WIDTH, TEXTURE_0, TEXTURE_1, TEXTURE_2, ShaderProgram, draw_circle_inefficiently};
use crate::video::VertexAttribDefinition;
use specs::prelude::*;
use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::{Shaders, MapRenderData, SpriteResource, PhysicsWorld, ElapsedTime};
use std::collections::HashMap;
use crate::cam::Camera;
use crate::components::controller::{ControllerComponent, WorldCoords};
use crate::components::{BrowserClient, FlyingNumberComponent, StrEffectComponent, FlyingNumberType};
use crate::components::char::{PhysicsComponent, CharacterStateComponent, ComponentRadius, SpriteBoundingRect, SpriteRenderDescriptorComponent, CharState, CharType, CharOutlook};
use crate::asset::str::KeyFrameType;
use crate::components::skills::skill::{Skills, SkillTargetType, SkillManifestationComponent};
use crate::common::v2_to_v3;
use crate::systems::ui::RenderUI;

/// The values that should be added to the sprite direction based on the camera
/// direction (the index is the camera direction, which is floor(angle/45)
pub const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];

// todo: Move it into GPU?
pub const ONE_SPRITE_PIXEL_SIZE_IN_3D: f32 = 1.0 / 35.0;


pub struct RenderDesktopClientSystem {
    rectangle_vao: VertexArray,
    capsule_vertex_arrays: HashMap<ComponentRadius, VertexArray>,
    // radius to vertexArray
    damage_render_sys: DamageRenderSystem,
    render_ui_sys: RenderUI,
}

impl RenderDesktopClientSystem {
    pub fn new() -> RenderDesktopClientSystem {
        let s: Vec<[f32; 2]> = vec![
            [0.0, 1.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 0.0]
        ];

        let capsule_vertex_arrays: HashMap<ComponentRadius, VertexArray> = [1, 2, 3, 4].iter().map(|radius| {
            let capsule_mesh = ncollide2d::procedural::circle(
                &(*radius as f32 * 0.5 * 2.0),
                32,
            );

            let coords = capsule_mesh.coords();
            (ComponentRadius(*radius), VertexArray::new(
                gl::LINE_LOOP,
                coords,
                coords.len(),
                vec![
                    VertexAttribDefinition {
                        number_of_components: 2,
                        offset_of_first_element: 0,
                    }
                ])
            )
        }).collect();

        RenderDesktopClientSystem {
            capsule_vertex_arrays,
            rectangle_vao: VertexArray::new(
                gl::TRIANGLE_STRIP,
                &s, 4, vec![
                    VertexAttribDefinition {
                        number_of_components: 2,
                        offset_of_first_element: 0,
                    }
                ]),
            damage_render_sys: DamageRenderSystem::new(),
            render_ui_sys: RenderUI::new(),
        }
    }

    pub fn render_for_controller<'a>(
        &self,
        controller: &mut ControllerComponent,
        physics_storage: &specs::ReadStorage<'a, PhysicsComponent>,
        physics_world: &specs::ReadExpect<'a, PhysicsWorld>,
        system_vars: &mut SystemVariables,
        char_state_storage: &specs::ReadStorage<'a, CharacterStateComponent>,
        entities: &specs::Entities<'a>,
        sprite_storage: &specs::ReadStorage<'a, SpriteRenderDescriptorComponent>,
        skill_storage: &specs::ReadStorage<'a, SkillManifestationComponent>, // TODO remove me
        str_effect_storage: &specs::ReadStorage<'a, StrEffectComponent>,
        updater: &specs::Write<'a, LazyUpdate>,
    ) {
        // Draw physics colliders
        for physics in (&physics_storage).join() {
            let mut matrix = Matrix4::<f32>::identity();
            let body = physics_world.rigid_body(physics.body_handle);
            if body.is_none() {
                continue;
            }
            let pos = body.unwrap().position().translation.vector;
            let pos = v3!(pos.x, 0.05, pos.y);
            matrix.prepend_translation_mut(&pos);
            let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::x()), std::f32::consts::FRAC_PI_2).to_homogeneous();
            matrix = matrix * rotation;

            let shader = system_vars.shaders.trimesh_shader.gl_use();
            shader.set_mat4("projection", &system_vars.matrices.projection);
            shader.set_mat4("view", &controller.view_matrix);
            shader.set_f32("alpha", 1.0);
            shader.set_mat4("model", &matrix);
            shader.set_vec4("color", &[1.0, 0.0, 1.0, 1.0]);
            self.capsule_vertex_arrays[&physics.radius].bind().draw();
        }

        let char_pos = char_state_storage.get(controller.char).unwrap().pos();
        render_client(
            &char_pos,
            &controller.camera,
            &controller.view_matrix,
            &system_vars.shaders,
            &system_vars.matrices.projection,
            &system_vars.map_render_data,
        );

        if let Some((skill_key, skill)) = controller.is_selecting_target() {
            let char_state = char_state_storage.get(controller.char).unwrap();
            draw_circle_inefficiently(&system_vars.shaders.trimesh_shader,
                                      &system_vars.matrices.projection,
                                      &controller.view_matrix,
                                      &char_state.pos(),
                                      0.0,
                                      skill.get_casting_range(),
                                      &[0.0, 1.0, 0.0, 1.0]);
            if skill.get_skill_target_type() == SkillTargetType::Area {
                let (skill_3d_pos, dir_vector) = Skills::limit_vector_into_range(
                    &char_pos,
                    &controller.mouse_world_pos,
                    skill.get_casting_range(),
                );
                skill.render_target_selection(
                    &skill_3d_pos,
                    &dir_vector,
                    &system_vars,
                    &controller.view_matrix,
                );
            }
        } else {
            let char_state = char_state_storage.get(controller.char).unwrap();
            if let CharState::CastingSkill(casting_info) = char_state.state() {
                let skill = casting_info.skill;
                skill.render_casting(
                    &char_pos,
                    casting_info,
                    system_vars,
                    &controller.view_matrix,
                );
            }
        }

        // Draw players
        for (entity_id, animated_sprite, char_state) in (entities,
                                                         sprite_storage,
                                                         char_state_storage).join() {
            // for autocompletion
            let char_state: &CharacterStateComponent = char_state;

            let pos = char_state.pos();
            let is_dead = char_state.state().is_dead();
            let color = char_state.statuses.calc_render_color();
            match char_state.outlook {
                CharOutlook::Player { job_id, head_index, sex } => {
                    let body_sprite = char_state.statuses.calc_render_sprite(
                        job_id,
                        head_index,
                        sex,
                        &system_vars.sprites,
                    );
                    let head_res = {
                        let sprites = &system_vars.sprites.head_sprites;
                        &sprites[sex as usize][head_index]
                    };
                    if controller.entity_below_cursor.filter(|it| *it == entity_id).is_some() {
                        let (pos_offset, _body_bounding_rect) = render_sprite(&system_vars,
                                                                              &animated_sprite,
                                                                              body_sprite,
                                                                              &controller.view_matrix,
                                                                              controller.yaw,
                                                                              &pos,
                                                                              [0, 0],
                                                                              true,
                                                                              1.1,
                                                                              is_dead,
                                                                              &[0.0, 0.0, 1.0, 0.4]);

                        let (_head_pos_offset, _head_bounding_rect) = render_sprite(&system_vars,
                                                                                    &animated_sprite,
                                                                                    head_res,
                                                                                    &controller.view_matrix,
                                                                                    controller.yaw,
                                                                                    &pos,
                                                                                    pos_offset,
                                                                                    false,
                                                                                    1.1,
                                                                                    is_dead,
                                                                                    &[0.0, 0.0, 1.0, 0.5]);
                    }

                    // todo: kell a pos_offset még mindig? (bounding rect)
                    let (pos_offset, mut body_bounding_rect) = render_sprite(&system_vars,
                                                                         &animated_sprite,
                                                                         body_sprite,
                                                                         &controller.view_matrix,
                                                                         controller.yaw,
                                                                         &pos,
                                                                         [0, 0],
                                                                         true,
                                                                         1.0,
                                                                         is_dead,
                                                                         &color);
                    let (head_pos_offset, head_bounding_rect) = render_sprite(&system_vars,
                                                                              &animated_sprite,
                                                                              head_res,
                                                                              &controller.view_matrix,
                                                                              controller.yaw,
                                                                              &pos,
                                                                              pos_offset,
                                                                              false,
                                                                              1.0,
                                                                              is_dead,
                                                                              &color);

                    body_bounding_rect.merge(&head_bounding_rect);

                    if !is_dead {
                        draw_health_bar(
                            &system_vars.shaders.trimesh2d_shader,
                            &self.rectangle_vao,
                            &system_vars.matrices.ortho,
                            controller.char == entity_id,
                            &char_state,
                            system_vars.time,
                            &body_bounding_rect
                        );
                    }

                    controller.bounding_rect_2d.insert(
                        entity_id, body_bounding_rect
                    );
                }
                CharOutlook::Monster(monster_id) => {
                    let body_res = {
                        let sprites = &system_vars.sprites.monster_sprites;
                        &sprites[&monster_id]
                    };
                    if controller.entity_below_cursor.filter(|it| *it == entity_id).is_some() {
                        let (_pos_offset, bounding_rect) = render_sprite(&system_vars,
                                                                         &animated_sprite,
                                                                         body_res,
                                                                         &controller.view_matrix,
                                                                         controller.yaw,
                                                                         &pos,
                                                                         [0, 0],
                                                                         true,
                                                                         1.1,
                                                                         is_dead,
                                                                         &[0.0, 0.0, 1.0, 0.5]);
                    }
                    let (_pos_offset, bounding_rect) = render_sprite(&system_vars,
                                                                     &animated_sprite,
                                                                     body_res,
                                                                     &controller.view_matrix,
                                                                     controller.yaw,
                                                                     &pos,
                                                                     [0, 0],
                                                                     true,
                                                                     1.0,
                                                                     is_dead,
                                                                     &color);
                    if !is_dead {
                        draw_health_bar(
                            &system_vars.shaders.trimesh2d_shader,
                            &self.rectangle_vao,
                            &system_vars.matrices.ortho,
                            controller.char == entity_id,
                            &char_state,
                            system_vars.time,
                            &bounding_rect
                        );
                    }

                    controller.bounding_rect_2d.insert(
                        entity_id, bounding_rect
                    );
                }
            }

            char_state.statuses.render(&char_state.pos(), system_vars, &controller.view_matrix);
        }

        for skill in (&skill_storage).join() {
            skill.render(system_vars, &controller.view_matrix);
        }

        for (entity_id, str_effect) in (entities, str_effect_storage).join() {
            if str_effect.die_at.has_passed(system_vars.time) {
                updater.remove::<StrEffectComponent>(entity_id);
            } else {
                RenderDesktopClientSystem::render_str(
                    &str_effect.effect,
                    str_effect.start_time,
                    &str_effect.pos,
                    system_vars,
                    &controller.view_matrix
                );
            }
        }
    }
}

impl<'a> specs::System<'a> for RenderDesktopClientSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::WriteStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, SpriteRenderDescriptorComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::ReadStorage<'a, SkillManifestationComponent>, // TODO remove me
        specs::ReadStorage<'a, StrEffectComponent>,
        specs::ReadExpect<'a, PhysicsWorld>,
        specs::Write<'a, LazyUpdate>,
        specs::ReadStorage<'a, FlyingNumberComponent>,
    );

    fn run(&mut self, (
        entities,
        mut controller_storage,
        mut browser_client_storage,
        physics_storage,
        sprite_storage,
        char_state_storage,
        mut system_vars,
        mut system_benchmark,
        skill_storage,
        str_effect_storage,
        physics_world,
        updater,
        numbers,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("RenderDesktopClientSystem");
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        for (mut controller, browser) in (&mut controller_storage, &mut browser_client_storage).join() {
            self.render_for_controller(
                &mut controller,
                &physics_storage,
                &physics_world,
                &mut system_vars,
                &char_state_storage,
                &entities,
                &sprite_storage,
                &skill_storage,
                &str_effect_storage,
                &updater,
            );

            self.damage_render_sys.run(
                &entities,
                &numbers,
                &char_state_storage,
                &controller,
                &system_vars,
                &updater,
            );

            self.render_ui_sys.run(
                &entities,
                &mut controller,
                &char_state_storage,
                &system_vars,
            );

            // now the back buffer contains the rendered image for this client
            unsafe {
                gl::ReadBuffer(gl::BACK);
                gl::ReadPixels(0, 0, VIDEO_WIDTH as i32, VIDEO_HEIGHT as i32,
                               gl::RGBA,
                               gl::UNSIGNED_BYTE, browser.offscreen.as_mut_ptr() as *mut gl::types::GLvoid);
            }
            let message = websocket::Message::binary(browser.offscreen.as_slice());
//                sent_bytes_per_second_counter += client.offscreen.len();
            // it is ok if it fails, the client might have disconnected but
            // ecs_world.maintain has not been executed yet to remove it from the world
            let _result = browser.websocket.lock().unwrap().send_message(&message);
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        for (mut controller, _not_browser) in (&mut controller_storage, !&browser_client_storage).join() {
            self.render_for_controller(
                &mut controller,
                &physics_storage,
                &physics_world,
                &mut system_vars,
                &char_state_storage,
                &entities,
                &sprite_storage,
                &skill_storage,
                &str_effect_storage,
                &updater,
            );

            self.damage_render_sys.run(
                &entities,
                &numbers,
                &char_state_storage,
                &controller,
                &system_vars,
                &updater,
            );

            self.render_ui_sys.run(
                &entities,
                &mut controller,
                &char_state_storage,
                &system_vars,
            );
        }
    }
}

fn draw_health_bar(
    shader: &ShaderProgram,
    vao: &VertexArray,
    ortho: &Matrix4<f32>,
    is_self: bool,
    char_state: &CharacterStateComponent,
    now: ElapsedTime,
    bounding_rect_2d: &SpriteBoundingRect,
) {
    let shader = shader.gl_use();
    shader.set_mat4("projection", &ortho);
    let vao = vao.bind();
    let bar_w = match char_state.typ {
        CharType::Player => 80,
        CharType::Minion => 70,
        _ => 100
    };
    let draw_rect = |x: i32, y: i32, w: i32, h: i32, color: &[f32; 4]| {
        let mut matrix = Matrix4::<f32>::identity();
        let spr_x = bounding_rect_2d.bottom_left[0];
        let spr_w = bounding_rect_2d.top_right[0] - bounding_rect_2d.bottom_left[0];
        let bar_x = spr_x as f32 + (spr_w as f32 / 2.0) - (bar_w as f32 / 2.0);
        let pos = Vector3::new(
            bar_x + x as f32,
            bounding_rect_2d.top_right[1] as f32 - 30.0 + y as f32,
            0.0,
        );
        matrix.prepend_translation_mut(&pos);
        shader.set_mat4("model", &matrix);
        shader.set_vec4("color", color);
        shader.set_vec2("size", &[w as f32, h as f32]);
        vao.draw();
    };


    let hp_percentage = char_state.hp as f32 / char_state.calculated_attribs.max_hp as f32;
    let health_color = if is_self {
        [0.29, 0.80, 0.11, 1.0] // for self, the health bar is green
    } else {
        [0.79, 0.00, 0.21, 1.0] // for enemies, red
        // [0.2, 0.46, 0.9] // for friends, blue
    };
    let mana_color = [0.23, 0.79, 0.88, 1.0];
    let bottom_bar_y = match char_state.typ {
        CharType::Player => {
            draw_rect(0, 0, bar_w, 9, &[0.0, 0.0, 0.0, 1.0]); // black border
            draw_rect(0, 0, bar_w, 5, &[0.0, 0.0, 0.0, 1.0]); // center separator
            let inner_w = ((bar_w - 2) as f32 * hp_percentage) as i32;
            draw_rect(1, 1, inner_w, 4, &health_color);
            draw_rect(1, 6, bar_w - 2, 2, &mana_color);
            9
        }
        _ => {
            draw_rect(0, 0, bar_w, 5, &[0.0, 0.0, 0.0, 1.0]); // black border
            let inner_w = ((bar_w - 2) as f32 * hp_percentage) as i32;
            draw_rect(1, 1, inner_w, 3, &health_color);
            5
        }
    };

    // draw status remaining time indicator
    if let Some(perc) = char_state.statuses.calc_largest_remaining_status_time_percent(now) {
        let orange = [1.0, 0.55, 0.0, 1.0];
        let w = bar_w - 4;
        draw_rect(2, bottom_bar_y + 2, w, 2, &[0.0, 0.0, 0.0, 1.0]); // black bg
        let inner_w = (w as f32 * (1.0 - perc)) as i32;
        draw_rect(2, bottom_bar_y + 2, inner_w, 2, &orange);
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
                     animation: &SpriteRenderDescriptorComponent,
                     sprite_res: &SpriteResource,
                     view: &Matrix4<f32>,
                     camera_yaw: f32,
                     pos: &Vector2<f32>,
                     pos_offset: [i32; 2],
                     is_main: bool,
                     size_multiplier: f32,
                     is_dead: bool,
                     color: &[f32; 4],
) -> ([i32; 2], SpriteBoundingRect) {
    let shader = system_vars.shaders.player_shader.gl_use();
    shader.set_mat4("projection", &system_vars.matrices.projection);
    shader.set_mat4("view", &view);
    shader.set_int("model_texture", 0);
    let binded_sprite_vertex_array = system_vars.map_render_data.centered_sprite_vertex_array.bind();

    let idx = {
        let cam_dir = (((camera_yaw / 45.0) + 0.5) as usize) % 8;
        animation.action_index + (animation.direction + DIRECTION_TABLE[cam_dir]) % 8
    };

    // TODO: if debug
    let action = sprite_res.action.actions.get(idx).or_else(|| {
        log::error!("Invalid action action index: {} idx: {}", animation.action_index, idx);
        Some(&sprite_res.action.actions[0])
    }).unwrap();
    let frame_index = if is_dead {
        action.frames.len() - 1
    } else {
        let frame_count = action.frames.len();
        let mut time_needed_for_one_frame = if let Some(duration) = animation.forced_duration {
            duration.div(frame_count as f32)
        } else {
            action.delay as f32 * (1.0 / animation.fps_multiplier) / 1000.0
        };
        time_needed_for_one_frame = if time_needed_for_one_frame == 0.0 { 0.1 } else { time_needed_for_one_frame };
        let elapsed_time = system_vars.time.elapsed_since(animation.animation_started);
        ((elapsed_time.div(time_needed_for_one_frame)) as usize % frame_count) as usize
    };
    let frame = &action.frames[frame_index];

    // TODO: refactor: ugly, valahol csak 1 layert kell irajzolni (player), valahol többet is (effektek)
    let mut width = 0.0;
    let mut height = 0.0;
    let mut offset = [0.0, 0.0];
    let matrix = {
        let mut matrix = Matrix4::<f32>::identity();
        let pos = v2_to_v3(&pos);
        matrix.prepend_translation_mut(&pos);
        shader.set_mat4("model", &matrix);
        matrix
    };
    for layer in frame.layers.iter() {
        if layer.sprite_frame_index < 0 {
            continue;
        }
        let sprite_texture = &sprite_res.textures[layer.sprite_frame_index as usize];

        width = sprite_texture.original_width as f32 * layer.scale[0] * size_multiplier;
        height = sprite_texture.original_height as f32 * layer.scale[1] * size_multiplier;
        sprite_texture.texture.bind(TEXTURE_0);

        offset = if !frame.positions.is_empty() && !is_main {
            [
                (pos_offset[0] - frame.positions[0][0]) as f32,
                (pos_offset[1] - frame.positions[0][1]) as f32
            ]
        } else {
            [0.0, 0.0]
        };
        offset = [layer.pos[0] as f32 + offset[0], layer.pos[1] as f32 + offset[1]];
        offset = [
            offset[0] as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D,
            offset[1] as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D - 0.1
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
    let anim_pos = frame.positions.get(0).map(|it| it.clone()).unwrap_or([0, 0]);

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

    if map_render_data.draw_ground {
        render_ground(shaders, projection_matrix, map_render_data, &model_view, &normal_matrix);
    }

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
                1.0 //0.3
            } else {
                model_render_data.alpha
            };
            shader.set_f32("alpha", alpha);
            for node_render_data in &model_render_data.model {
                // TODO: optimize this
                for face_render_data in node_render_data {
                    face_render_data.texture.bind(TEXTURE_0);
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
    map_render_data.texture_atlas.bind(TEXTURE_0);
    shader.set_int("gnd_texture_atlas", 0);
    map_render_data.tile_color_texture.bind(TEXTURE_1);
    shader.set_int("tile_color_texture", 1);
    map_render_data.lightmap_texture.bind(TEXTURE_2);
    shader.set_int("lightmap_texture", 2);
    shader.set_int("use_tile_color", if map_render_data.use_tile_colors { 1 } else { 0 });
    shader.set_int("use_lightmap", if map_render_data.use_lightmaps { 1 } else { 0 });
    shader.set_int("use_lighting", if map_render_data.use_lighting { 1 } else { 0 });
    map_render_data.ground_vertex_array.bind().draw();
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
        return digits;
    }
}

impl DamageRenderSystem {
    pub fn run(
        &self,
        entities: &specs::Entities,
        numbers: &specs::ReadStorage<FlyingNumberComponent>,
        char_state_storage: &specs::ReadStorage<CharacterStateComponent>,
        controller: &ControllerComponent,
        system_vars: &specs::WriteExpect<SystemVariables>,
        updater: &specs::Write<LazyUpdate>,
    ) {
        unsafe {
            gl::Disable(gl::DEPTH_TEST);
        }
        // for autocompletion
        let controller: &ControllerComponent = controller;

        for (entity_id, number) in (entities, numbers).join() {
            let mut matrix = Matrix4::<f32>::identity();
            let mut pos = Vector3::new(number.start_pos.x, 1.0, number.start_pos.y);

            let (width, height) = match number.typ {
                FlyingNumberType::Poison |
                FlyingNumberType::Heal |
                FlyingNumberType::Damage |
                FlyingNumberType::Mana |
                FlyingNumberType::Crit => {
                    (
                        DamageRenderSystem::get_digits(number.value).len() as f32,
                        1.0
                    )
                }
                FlyingNumberType::Block => {
                    (
                        system_vars.texts.attack_blocked.width as f32,
                        system_vars.texts.attack_blocked.height as f32,
                    )
                }
                FlyingNumberType::Absorb => {
                    (
                        system_vars.texts.attack_absorbed.width as f32,
                        system_vars.texts.attack_absorbed.height as f32,
                    )
                }
            };

            let perc = system_vars.time.elapsed_since(number.start_time).div(number.duration as f32);
            // TODO: don't render more than 1 damage in a single frame for the same target
            let (size_x, size_y) = match number.typ {
                FlyingNumberType::Heal => {
                    // follow the target
                    let real_pos = char_state_storage
                        .get(number.target_entity_id)
                        .map(|it| it.pos())
                        .unwrap_or(number.start_pos);
                    pos.x = real_pos.x;
                    pos.z = real_pos.y;
                    let size = ((1.0 - perc * 3.0) * 1.5).max(0.4);
                    pos.x -= width * size / 2.0;
                    let y_offset = if perc < 0.3 { 0.0 } else { (perc - 0.3) * 3.0 };
                    pos.y += 2.0 + y_offset;
                    // a small hack to mitigate the distortion effect of perspective projection
                    // at the edge of the screens
                    pos.z -= y_offset;
                    (size, size)
                }
                FlyingNumberType::Damage => {
                    pos.x += perc * 6.0;
                    pos.z -= perc * 4.0;
                    pos.y += 2.0 +
                        (-std::f32::consts::FRAC_PI_2 + (std::f32::consts::PI * (0.5 + perc * 1.5))).sin() * 5.0;
                    let size = (1.0 - perc) * 1.0;
                    (size, size)
                }
                FlyingNumberType::Poison => {
                    let real_pos = char_state_storage
                        .get(number.target_entity_id)
                        .map(|it| it.pos())
                        .unwrap_or(number.start_pos);
                    pos.x = real_pos.x;
                    pos.z = real_pos.y;
                    let size = 0.4;
                    pos.x -= width * size / 2.0;
                    let y_offset = (perc - 0.3) * 3.0;
                    pos.y += 2.0 + y_offset;
                    pos.z -= y_offset;
                    (size, size)
                }
                FlyingNumberType::Block | FlyingNumberType::Absorb => {
                    let real_pos = char_state_storage
                        .get(number.target_entity_id)
                        .map(|it| it.pos())
                        .unwrap_or(number.start_pos);
                    pos.x = real_pos.x;
                    pos.z = real_pos.y;
                    let size_x = width * ONE_SPRITE_PIXEL_SIZE_IN_3D;
                    let size_y = height * ONE_SPRITE_PIXEL_SIZE_IN_3D;
//                        pos.x -= size_x / 2.0;
                    let y_offset = (perc - 0.3) * 3.0;
                    pos.y += 2.0 + y_offset;
                    pos.z -= y_offset;
                    (size_x, size_y)
                }
                _ => {
                    pos.y += 4.0 * perc;
                    pos.z -= 2.0 * perc;
                    pos.x += 2.0 * perc;
                    let size = (1.0 - perc) * 4.0;
                    (size, size)
                }
            };
            matrix.prepend_translation_mut(&pos);
            let shader = system_vars.shaders.sprite_shader.gl_use();
            shader.set_vec2("size", &[
                size_x,
                size_y
            ]);
            shader.set_mat4("model", &matrix);
            shader.set_vec3("color", &number.typ.color(controller.char == number.target_entity_id));
            shader.set_mat4("projection", &system_vars.matrices.projection);
            shader.set_mat4("view", &controller.view_matrix);
            shader.set_int("model_texture", 0);
            shader.set_f32("alpha", 1.3 - (perc + 0.3 * perc));

            match number.typ {
                FlyingNumberType::Poison |
                FlyingNumberType::Heal |
                FlyingNumberType::Damage |
                FlyingNumberType::Mana |
                FlyingNumberType::Crit => {
                    system_vars.sprites.numbers.bind(TEXTURE_0);
                    self.create_number_vertex_array(number.value).bind().draw();
                }
                FlyingNumberType::Block => {
                    system_vars.texts.attack_blocked.bind(TEXTURE_0);
                    system_vars.map_render_data.centered_sprite_vertex_array.bind().draw();
                }
                FlyingNumberType::Absorb => {
                    system_vars.texts.attack_absorbed.bind(TEXTURE_0);
                    system_vars.map_render_data.centered_sprite_vertex_array.bind().draw();
                }
            };

            if number.die_at.has_passed(system_vars.time) {
                updater.remove::<FlyingNumberComponent>(entity_id);
            }
        }
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
        }
    }
}

//impl<'a> specs::System<'a> for DamageRenderSystem {
//    type SystemData = (
//        specs::Entities<'a>,
//        specs::ReadStorage<'a, FlyingNumberComponent>,
//        &'a specs::ReadStorage<'a, CharacterStateComponent>,
//        specs::ReadStorage<'a, ControllerComponent>,
//        specs::ReadExpect<'a, SystemVariables>,
//        specs::WriteExpect<'a, SystemFrameDurations>,
//        specs::Write<'a, LazyUpdate>,
//    );
//    fn run(&mut self, (
//        entities,
//        numbers,
//        char_state_storage,
//        controller_storage,
//        system_vars,
//        mut system_benchmark,
//        updater,
//    ): Self::SystemData) {
//    }
//}

impl DamageRenderSystem {
    pub fn create_number_vertex_array(&self, number: u32) -> VertexArray {
        let digits = DamageRenderSystem::get_digits(number);
        // create vbo based on the numbers
        let mut width = 0.0;
        let mut vertices = vec![];
        digits.iter().for_each(|&digit| {
            let digit = digit as usize;
            vertices.push([width - 0.5, 0.5, self.texture_u_coords[digit], 0.0]);
            vertices.push([width + 0.5, 0.5, self.texture_u_coords[digit] + self.single_digit_u_coord, 0.0]);
            vertices.push([width - 0.5, -0.5, self.texture_u_coords[digit], 1.0]);
            vertices.push([width + 0.5, 0.5, self.texture_u_coords[digit] + self.single_digit_u_coord, 0.0]);
            vertices.push([width - 0.5, -0.5, self.texture_u_coords[digit], 1.0]);
            vertices.push([width + 0.5, -0.5, self.texture_u_coords[digit] + self.single_digit_u_coord, 1.0]);
            width += 1.0;
        });
        return VertexArray::new(
            gl::TRIANGLES,
            &vertices, vertices.len(), vec![
                VertexAttribDefinition {
                    number_of_components: 2,
                    offset_of_first_element: 0,
                }, VertexAttribDefinition { // uv
                    number_of_components: 2,
                    offset_of_first_element: 2,
                }
            ]);
    }
}

impl RenderDesktopClientSystem {
    pub fn render_str(
        effect_name: &str,
        start_time: ElapsedTime,
        world_pos: &WorldCoords,
        system_vars: &mut SystemVariables,
        view_matrix: &Matrix4<f32>,
    ) {
        unsafe {
            gl::Disable(gl::DEPTH_TEST);
        }
        let shader = system_vars.shaders.str_effect_shader.gl_use();
        shader.set_mat4("projection", &system_vars.matrices.projection);
        shader.set_mat4("view", view_matrix);
        shader.set_int("model_texture", 0);

        let str_file = &system_vars.map_render_data.str_effects[effect_name];
        let seconds_needed_for_one_frame = 1.0 / str_file.fps as f32;
        let max_key = str_file.max_key;
        let key_index = system_vars.time.elapsed_since(start_time).div(seconds_needed_for_one_frame) as i32 % max_key as i32;

        let mut from_id = None;
        let mut to_id = None;
        let mut last_source_id = 0;
        let mut last_frame_id = 0;
        for layer in str_file.layers.iter() {
            for (i, key_frame) in layer.key_frames.iter().enumerate() {
                if key_frame.frame <= key_index {
                    match key_frame.typ {
                        KeyFrameType::Start => from_id = Some(i),
                        KeyFrameType::End => to_id = Some(i),
                    };
                }
                last_frame_id = last_frame_id.max(key_frame.frame);
                if key_frame.typ == KeyFrameType::Start {
                    last_source_id = last_source_id.max(key_frame.frame);
                }
            }
            if from_id.is_none() || to_id.is_none() || last_frame_id < key_index {
                continue;
            }
            let from_id = from_id.unwrap();
            let to_id = to_id.unwrap();
            if from_id >= layer.key_frames.len() || to_id >= layer.key_frames.len() {
                continue;
            }
            let from_frame = &layer.key_frames[from_id];
            let to_frame = &layer.key_frames[to_id];

            let (color, pos, uv, xy, angle) = if to_id != from_id + 1 || to_frame.frame != from_frame.frame {
                // no other source
                if last_source_id <= from_frame.frame {
                    continue;
                }
                (from_frame.color, from_frame.pos, from_frame.uv, from_frame.xy, from_frame.angle)
            } else {
                let delta = (key_index - from_frame.frame) as f32;
                // morphing
                let color = [
                    from_frame.color[0] + to_frame.color[0] * delta,
                    from_frame.color[1] + to_frame.color[1] * delta,
                    from_frame.color[2] + to_frame.color[2] * delta,
                    from_frame.color[3] + to_frame.color[3] * delta,
                ];
                let xy = [
                    from_frame.xy[0] + to_frame.xy[0] * delta,
                    from_frame.xy[1] + to_frame.xy[1] * delta,
                    from_frame.xy[2] + to_frame.xy[2] * delta,
                    from_frame.xy[3] + to_frame.xy[3] * delta,
                    from_frame.xy[4] + to_frame.xy[4] * delta,
                    from_frame.xy[5] + to_frame.xy[5] * delta,
                    from_frame.xy[6] + to_frame.xy[6] * delta,
                    from_frame.xy[7] + to_frame.xy[7] * delta,
                ];
                let angle = from_frame.angle + to_frame.angle * delta;
                let pos = [
                    from_frame.pos[0] + to_frame.pos[0] * delta,
                    from_frame.pos[1] + to_frame.pos[1] * delta,
                ];
                (color, pos, from_frame.uv, xy, angle)
            };


            let matrix = {
                let mut matrix = Matrix4::<f32>::identity();
                matrix.prepend_translation_mut(&v2_to_v3(world_pos));
                let rotation = Rotation3::from_axis_angle(
                    &nalgebra::Unit::new_normalize(Vector3::z()),
                    -angle)
                    .to_homogeneous();
                matrix = matrix * rotation;
                shader.set_mat4("model", &matrix);
                matrix
            };

            let offset = [pos[0] - 320.0, pos[1] - 320.0];

            shader.set_vec2("offset", &offset);
            shader.set_vec4("color", &color);
            system_vars.str_effect_vao[0] = xy[0];
            system_vars.str_effect_vao[1] = xy[4];
            system_vars.str_effect_vao[4] = xy[1];
            system_vars.str_effect_vao[5] = xy[5];
            system_vars.str_effect_vao[8] = xy[3];
            system_vars.str_effect_vao[9] = xy[7];
            system_vars.str_effect_vao[12] = xy[2];
            system_vars.str_effect_vao[13] = xy[6];

            unsafe {
                gl::BlendFunc(from_frame.src_alpha, from_frame.dst_alpha);
            }
            str_file.textures[from_frame.texture_index].bind(TEXTURE_0);
            system_vars.str_effect_vao.bind().draw();
        }
        unsafe {
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEPTH_TEST);
        }
    }
}